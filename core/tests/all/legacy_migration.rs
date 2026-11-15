//! Ported from `tests/Sobreiro.Tests/LegacyMigrationTests.cs`, with tests of what happens to
//! the Run value whatever the folder does, and of a second run.
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use sobreiro_core::migration::legacy_migration::{self, OLD_MUTEX_NAME, STILL_RUNNING_MESSAGE};
use sobreiro_core::migration::{
    DataFolderOutcome, MigrationContext, MigrationFiles, MigrationResult, RunValueOutcome,
    RunValues,
};
use sobreiro_core::startup::auto_start_command;

const OLD: &str = r"C:\Local\OpenHeadroom";
const NEW: &str = r"C:\Local\Sobreiro";
const EXE: &str = r"C:\Apps\Sobreiro\Sobreiro.exe";

#[derive(Default)]
struct FakeFiles {
    directories: RefCell<HashSet<PathBuf>>,
    move_fails: Cell<bool>,
    copy_misses: Cell<usize>,
    log: RefCell<Vec<String>>,
}

impl FakeFiles {
    fn add(&self, path: &str) {
        self.directories.borrow_mut().insert(PathBuf::from(path));
    }

    fn has(&self, path: &str) -> bool {
        self.directories.borrow().contains(Path::new(path))
    }

    fn log(&self) -> Vec<String> {
        self.log.borrow().clone()
    }
}

impl MigrationFiles for FakeFiles {
    fn directory_exists(&self, path: &Path) -> bool {
        self.directories.borrow().contains(path)
    }

    fn move_directory(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.log
            .borrow_mut()
            .push(format!("move {} -> {}", from.display(), to.display()));
        if self.move_fails.get() {
            return Err(io::Error::other("a file is in use"));
        }
        let mut directories = self.directories.borrow_mut();
        directories.remove(from);
        directories.insert(to.to_path_buf());
        Ok(())
    }

    fn copy_directory(&self, from: &Path, to: &Path) -> usize {
        self.log
            .borrow_mut()
            .push(format!("copy {} -> {}", from.display(), to.display()));
        self.directories.borrow_mut().insert(to.to_path_buf());
        self.copy_misses.get()
    }
}

#[derive(Default)]
struct FakeRun {
    values: RefCell<BTreeMap<String, String>>,
    get_fails: Cell<bool>,
    set_fails: Cell<bool>,
    delete_fails: Cell<bool>,
}

impl FakeRun {
    fn put(&self, name: &str, command: &str) {
        self.values
            .borrow_mut()
            .insert(name.to_string(), command.to_string());
    }

    fn names(&self) -> Vec<String> {
        self.values.borrow().keys().cloned().collect()
    }
}

fn denied() -> io::Error {
    io::ErrorKind::PermissionDenied.into()
}

impl RunValues for FakeRun {
    fn get(&self, name: &str) -> io::Result<Option<String>> {
        if self.get_fails.get() {
            return Err(denied());
        }
        Ok(self.values.borrow().get(name).cloned())
    }

    fn set(&self, name: &str, command: &str) -> io::Result<()> {
        if self.set_fails.get() {
            return Err(denied());
        }
        self.put(name, command);
        Ok(())
    }

    fn delete(&self, name: &str) -> io::Result<()> {
        if self.delete_fails.get() {
            return Err(denied());
        }
        self.values.borrow_mut().remove(name);
        Ok(())
    }
}

#[derive(Default)]
struct Setup {
    files: FakeFiles,
    run: FakeRun,
}

impl Setup {
    fn run(&self) -> MigrationResult {
        self.run_with(true, false)
    }

    fn run_with(&self, is_default: bool, old_running: bool) -> MigrationResult {
        legacy_migration::run(&MigrationContext {
            is_default_data_directory: is_default,
            old_data_directory: Path::new(OLD),
            new_data_directory: Path::new(NEW),
            exe_path: Path::new(EXE),
            old_app_running: &|| old_running,
            files: &self.files,
            run_values: &self.run,
        })
    }
}

#[test]
fn moves_the_old_folder_to_the_new_name() {
    let s = Setup::default();
    s.files.add(OLD);
    let result = s.run();

    assert_eq!(result.data_folder, DataFolderOutcome::Moved);
    assert_eq!(s.files.log(), [format!("move {OLD} -> {NEW}")]);
    assert!(s.files.has(NEW));
    assert!(!s.files.has(OLD));
}

#[test]
fn nothing_to_move_without_an_old_folder() {
    let s = Setup::default();
    assert_eq!(s.run().data_folder, DataFolderOutcome::NothingToMove);
    assert!(s.files.log().is_empty());
}

#[test]
fn leaves_both_folders_alone_when_both_exist() {
    let s = Setup::default();
    s.files.add(OLD);
    s.files.add(NEW);
    let result = s.run();

    assert_eq!(result.data_folder, DataFolderOutcome::BothExist);
    assert!(s.files.log().is_empty());
}

#[test]
fn does_nothing_when_a_data_directory_override_is_in_use() {
    let s = Setup::default();
    s.files.add(OLD);
    s.run.put("OpenHeadroom", r#""C:\old\OpenHeadroom.exe""#);
    let result = s.run_with(false, true);

    assert_eq!(result.data_folder, DataFolderOutcome::NotDefault);
    assert!(!result.old_app_running);
    assert!(s.files.log().is_empty());
    assert_eq!(s.run.names(), ["OpenHeadroom"]);
}

#[test]
fn a_move_that_fails_copies_what_it_can_and_keeps_the_old_folder() {
    let s = Setup::default();
    s.files.add(OLD);
    s.files.move_fails.set(true);
    s.files.copy_misses.set(2);
    let result = s.run();

    assert_eq!(result.data_folder, DataFolderOutcome::CopiedAfterMoveFailed);
    assert_eq!(result.files_not_copied, 2);
    assert!(s.files.has(OLD));
    assert_eq!(
        s.files.log().last().unwrap(),
        &format!("copy {OLD} -> {NEW}")
    );
}

#[test]
fn run_value_is_replaced_by_one_for_this_exe() {
    let s = Setup::default();
    s.run
        .put("OpenHeadroom", r#""C:\Apps\OpenHeadroom\OpenHeadroom.exe""#);
    let result = s.run();

    assert_eq!(result.run_value, RunValueOutcome::Migrated);
    assert!(!s.run.values.borrow().contains_key("OpenHeadroom"));
    assert_eq!(
        s.run.values.borrow()["Sobreiro"],
        auto_start_command::command_for(Path::new(EXE))
    );
}

#[test]
fn run_value_absent_changes_nothing() {
    let s = Setup::default();
    let result = s.run();

    assert_eq!(result.run_value, RunValueOutcome::Absent);
    assert!(s.run.values.borrow().is_empty());
}

#[test]
fn a_registry_failure_leaves_the_old_value_and_does_not_throw() {
    let s = Setup::default();
    s.run.put("OpenHeadroom", "x");
    s.run.set_fails.set(true);
    let result = s.run();

    assert_eq!(result.run_value, RunValueOutcome::Failed);
    assert!(s.run.values.borrow().contains_key("OpenHeadroom"));
}

#[test]
fn old_app_running_means_nothing_is_migrated() {
    let s = Setup::default();
    s.files.add(OLD);
    s.run.put("OpenHeadroom", "x");
    let result = s.run_with(true, true);

    assert!(result.old_app_running);
    assert!(s.files.log().is_empty());
    assert_eq!(s.run.names(), ["OpenHeadroom"]);
}

#[test]
fn still_running_message_and_old_mutex_name_are_fixed() {
    assert_eq!(
        STILL_RUNNING_MESSAGE,
        "OpenHeadroom is still running. Quit it from its tray menu, then start Sobreiro again."
    );
    assert_eq!(OLD_MUTEX_NAME, r"Local\OpenHeadroom.Tray");
}

#[test]
fn the_run_value_moves_whatever_the_folder_does() {
    for (old, new, move_fails, outcome) in [
        (false, false, false, DataFolderOutcome::NothingToMove),
        (true, true, false, DataFolderOutcome::BothExist),
        (true, false, false, DataFolderOutcome::Moved),
        (true, false, true, DataFolderOutcome::CopiedAfterMoveFailed),
    ] {
        let s = Setup::default();
        if old {
            s.files.add(OLD);
        }
        if new {
            s.files.add(NEW);
        }
        s.files.move_fails.set(move_fails);
        s.run.put("OpenHeadroom", "x");
        let result = s.run();

        assert_eq!(result.data_folder, outcome);
        assert_eq!(result.run_value, RunValueOutcome::Migrated, "{outcome:?}");
        assert_eq!(s.run.names(), ["Sobreiro"]);
    }
}

#[test]
fn a_run_value_that_cannot_be_read_is_a_failure_and_nothing_is_written() {
    let s = Setup::default();
    s.run.put("OpenHeadroom", "x");
    s.run.get_fails.set(true);
    let result = s.run();

    assert_eq!(result.run_value, RunValueOutcome::Failed);
    assert_eq!(s.run.names(), ["OpenHeadroom"]);
}

#[test]
fn an_old_value_that_cannot_be_deleted_is_a_failure_and_both_values_stay() {
    let s = Setup::default();
    s.run.put("OpenHeadroom", "x");
    s.run.delete_fails.set(true);
    let result = s.run();

    assert_eq!(result.run_value, RunValueOutcome::Failed);
    assert_eq!(s.run.names(), ["OpenHeadroom", "Sobreiro"]);
}

#[test]
fn a_second_run_finds_nothing_to_do() {
    let s = Setup::default();
    s.files.add(OLD);
    s.run.put("OpenHeadroom", "x");
    s.run();
    let second = s.run();

    assert_eq!(
        second,
        MigrationResult {
            old_app_running: false,
            data_folder: DataFolderOutcome::NothingToMove,
            files_not_copied: 0,
            run_value: RunValueOutcome::Absent,
        }
    );
    assert_eq!(s.files.log().len(), 1);
}
