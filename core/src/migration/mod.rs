//! The first run after the rename from OpenHeadroom: the data folder and the Start with Windows
//! entry move to the new name. The shell hands in the disk, the registry and whether the old app
//! is running.
pub mod disk_migration_files;
pub mod legacy_migration;

pub use disk_migration_files::DiskMigrationFiles;
pub use legacy_migration::{
    DataFolderOutcome, MigrationContext, MigrationFiles, MigrationResult, RunValueOutcome,
    RunValues,
};
