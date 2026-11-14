//! SPIKE (shell candidate B): where the tray icons and the task buttons are, asked of UI Automation through
//! the system's own COM interface. Slow, so it runs on one thread of its own that lives as long as the app
//! and belongs to COM's multithreaded apartment. Every COM object is released when its value is dropped.
use crate::{native, spike, trace};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Mutex, OnceLock};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationCacheRequest, IUIAutomationElement, TreeScope, UIA_BoundingRectanglePropertyId,
    UIA_ClassNamePropertyId, UIA_ProcessIdPropertyId,
};

const FRAME_CLASS: &str = "Taskbar.TaskbarFrameAutomationPeer";

type Rect = (i32, i32, i32, i32);

/// What the taskbar answered: its own bounds, each grandchild's class, bounds and process, and the task buttons.
pub struct Answer {
    pub bounds: Rect,
    pub items: Vec<(String, Rect, i32)>,
    pub buttons: Vec<Rect>,
}

/// How many questions have been put, for the trace.
pub static ASKED: AtomicU32 = AtomicU32::new(0);

struct Question {
    taskbar: isize,
    done: Box<dyn FnOnce(Option<Answer>) + Send>,
}

static QUEUE: OnceLock<Mutex<Sender<Question>>> = OnceLock::new();

/// Puts the question to the probe's thread. `done` runs on that thread: hand the answer on from there.
pub fn ask(taskbar: HWND, done: impl FnOnce(Option<Answer>) + Send + 'static) {
    let queue = QUEUE.get_or_init(|| {
        let (sender, receiver) = channel::<Question>();
        let _ = std::thread::Builder::new().name("taskbar probe".into()).spawn(move || {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            }
            let mut automation: Option<IUIAutomation> = None;
            for question in receiver {
                ASKED.fetch_add(1, Ordering::Relaxed);
                let answer = query(&mut automation, HWND(question.taskbar as *mut _));
                (question.done)(answer);
            }
        });
        Mutex::new(sender)
    });
    if let Ok(sender) = queue.lock() {
        let _ = sender.send(Question { taskbar: taskbar.0 as isize, done: Box::new(done) });
    }
}

fn query(automation: &mut Option<IUIAutomation>, taskbar: HWND) -> Option<Answer> {
    let bounds = native::window_rect(taskbar)?;
    match ask_tree(automation, taskbar) {
        Ok((items, buttons)) => Some(Answer { bounds: (bounds.left, bounds.top, bounds.right, bounds.bottom), items, buttons }),
        Err(error) => {
            trace!("taskbar probe failed: 0x{:x}", error.code().0);
            if spike::env("SOBREIRO_SPIKE_UIA_KEEP").is_none() {
                *automation = None;
            }
            None
        }
    }
}

#[allow(clippy::type_complexity)]
fn ask_tree(automation: &mut Option<IUIAutomation>, taskbar: HWND) -> windows::core::Result<(Vec<(String, Rect, i32)>, Vec<Rect>)> {
    let automation = match automation {
        Some(automation) => automation,
        None => automation.insert(unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)? }),
    };
    unsafe {
        let cache = automation.CreateCacheRequest()?;
        cache.AddProperty(UIA_ClassNamePropertyId)?;
        cache.AddProperty(UIA_BoundingRectanglePropertyId)?;
        cache.AddProperty(UIA_ProcessIdPropertyId)?;
        cache.SetTreeScope(TreeScope(1 | 2))?; // the element and its children

        let mut items = Vec::new();
        let mut buttons = Vec::new();
        let root = automation.ElementFromHandleBuildCache(taskbar, &cache)?;
        for level1 in children(&root) {
            for child in children(&level1.BuildUpdatedCache(&cache)?) {
                let class_name = child.CachedClassName().map(|s| s.to_string()).unwrap_or_default();
                let frame = class_name == FRAME_CLASS;
                items.push((class_name, rect(&child), child.CachedProcessId().unwrap_or(0)));
                if frame {
                    buttons.extend(children(&updated(&child, &cache)?).iter().map(rect));
                }
            }
        }
        Ok((items, buttons))
    }
}

unsafe fn updated(element: &IUIAutomationElement, cache: &IUIAutomationCacheRequest) -> windows::core::Result<IUIAutomationElement> {
    unsafe { element.BuildUpdatedCache(cache) }
}

fn children(element: &IUIAutomationElement) -> Vec<IUIAutomationElement> {
    unsafe {
        let Ok(array) = element.GetCachedChildren() else { return Vec::new() };
        let count = array.Length().unwrap_or(0);
        (0..count).filter_map(|i| array.GetElement(i).ok()).collect()
    }
}

// An element without bounds answers with an error or an empty rectangle: both become zeros.
fn rect(element: &IUIAutomationElement) -> Rect {
    match unsafe { element.CachedBoundingRectangle() } {
        Ok(r) => (r.left, r.top, r.right, r.bottom),
        Err(_) => (0, 0, 0, 0),
    }
}
