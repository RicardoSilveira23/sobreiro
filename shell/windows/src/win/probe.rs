//! Where the taskbar's tray icons and task buttons are, read from UI Automation. The question is
//! slow, so it is asked on a worker thread of its own, which keeps COM (multithreaded) and one
//! UI Automation client for its whole life. Questions are answered in the order they are asked.
#![allow(unsafe_code)]

use crate::win::taskbar;
use sobreiro_core::display::flyout_placement::PixelRect;
use sobreiro_core::taskbar::taskbar_layout::{self, TaskbarAnchors};
use std::ffi::c_void;
use std::sync::mpsc::{self, Receiver, SendError, Sender};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, TreeScope, TreeScope_Children,
    TreeScope_Element, UIA_BoundingRectanglePropertyId, UIA_ClassNamePropertyId,
    UIA_ProcessIdPropertyId,
};

/// The class of the element whose children are the task buttons.
const FRAME_CLASS: &str = "Taskbar.TaskbarFrameAutomationPeer";

/// What is done with an answer, on the worker thread.
pub type Reply = Box<dyn FnOnce(Option<TaskbarAnchors>) + Send>;

struct Question {
    taskbar: isize,
    reply: Reply,
}

/// The worker that asks UI Automation. It starts at the first question, and again if it ended.
#[derive(Default)]
pub struct Prober {
    questions: Option<Sender<Question>>,
}

impl Prober {
    /// Asks where the parts of `taskbar` are; `reply` gets the anchors, or None when the taskbar
    /// cannot be read (auto-hidden, restarting, no tray items) or anything fails. False when the
    /// question could not be sent, and `reply` will never be called.
    pub fn ask(&mut self, taskbar: isize, reply: Reply) -> bool {
        let mut question = Question { taskbar, reply };
        if let Some(questions) = &self.questions {
            match questions.send(question) {
                Ok(()) => return true,
                Err(SendError(back)) => question = back,
            }
        }
        self.questions = None;
        let (questions, answers_to) = mpsc::channel();
        let started = std::thread::Builder::new()
            .name("taskbar-probe".into())
            .spawn(move || work(answers_to));
        if started.is_err() {
            return false;
        }
        let sent = questions.send(question).is_ok();
        self.questions = Some(questions);
        sent
    }
}

fn work(questions: Receiver<Question>) {
    // SAFETY: starts COM on this thread, which is the worker's own; undone below, after the last
    // interface is dropped.
    let started = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
    {
        let mut automation: Option<IUIAutomation> = None;
        for question in questions.iter() {
            if automation.is_none() {
                automation = client();
            }
            let answer = automation
                .as_ref()
                .and_then(|automation| query(automation, question.taskbar));
            (question.reply)(answer);
        }
    }
    if started {
        // SAFETY: matches the CoInitializeEx above, on the same thread.
        unsafe { CoUninitialize() };
    }
}

fn client() -> Option<IUIAutomation> {
    // SAFETY: makes the system's UI Automation client, in this process, on a thread where COM
    // has been started.
    unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }.ok()
}

/// The taskbar's children and their children, as 1.1 reads them: each child of the taskbar's
/// element is read again with its own children, and the task buttons are the children of the
/// frame among them. None on any failure.
fn query(automation: &IUIAutomation, taskbar: isize) -> Option<TaskbarAnchors> {
    let bounds = taskbar::window_rect(taskbar)?;
    let mut items: Vec<(String, PixelRect, u32)> = Vec::new();
    let mut buttons: Vec<PixelRect> = Vec::new();
    // SAFETY: COM calls on interfaces this thread made, with arguments that live across each
    // call; the handle is only read by UI Automation, which fails on one that is no window.
    unsafe {
        let cache = automation.CreateCacheRequest().ok()?;
        cache
            .SetTreeScope(TreeScope(TreeScope_Element.0 | TreeScope_Children.0))
            .ok()?;
        cache.AddProperty(UIA_ClassNamePropertyId).ok()?;
        cache.AddProperty(UIA_BoundingRectanglePropertyId).ok()?;
        cache.AddProperty(UIA_ProcessIdPropertyId).ok()?;
        let root = automation
            .ElementFromHandleBuildCache(HWND(taskbar as *mut c_void), &cache)
            .ok()?;
        for level1 in cached_children(&root)? {
            let level1 = level1.BuildUpdatedCache(&cache).ok()?;
            for child in cached_children(&level1)? {
                let class_name = child.CachedClassName().ok()?.to_string();
                let rect = to_pixels(child.CachedBoundingRectangle().ok()?);
                let process_id = child.CachedProcessId().ok()? as u32;
                let is_frame = class_name == FRAME_CLASS;
                items.push((class_name, rect, process_id));
                if !is_frame {
                    continue;
                }
                let frame = child.BuildUpdatedCache(&cache).ok()?;
                for button in cached_children(&frame)? {
                    buttons.push(to_pixels(button.CachedBoundingRectangle().ok()?));
                }
            }
        }
    }
    taskbar_layout::anchors(
        bounds,
        items
            .iter()
            .map(|(class_name, rect, process_id)| (class_name.as_str(), *rect, *process_id)),
        buttons,
        std::process::id(),
    )
}

/// The cached children of an element, or None when UI Automation fails, as 1.1's query throws
/// then. An element without children gives no array (a success with nothing in it), not an
/// empty one; that alone reads as no children.
fn cached_children(element: &IUIAutomationElement) -> Option<Vec<IUIAutomationElement>> {
    // SAFETY: COM calls on an interface this thread holds; the index stays below the length.
    unsafe {
        let array = match element.GetCachedChildren() {
            Ok(array) => array,
            Err(error) if error.code().is_ok() => return Some(Vec::new()),
            Err(_) => return None,
        };
        let length = array.Length().ok()?;
        (0..length)
            .map(|index| array.GetElement(index).ok())
            .collect()
    }
}

fn to_pixels(rect: RECT) -> PixelRect {
    PixelRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}
