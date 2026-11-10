use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::instances::{InstanceConfig, InstanceName};

use crate::instances::instance::config_path;
use crate::instances::{Instance, InstanceError, recognize_install};
use crate::paths::DataDir;

#[derive(Clone)]
pub struct InstanceStore {
    data_dir: DataDir,
}

impl InstanceStore {
    pub fn new(data_dir: DataDir) -> Self {
        Self { data_dir }
    }

    pub fn data_dir(&self) -> &DataDir {
        &self.data_dir
    }

    pub fn create(
        &self,
        name: InstanceName,
        config: InstanceConfig,
    ) -> Result<Instance, InstanceError> {
        let instances_dir = self.data_dir.instances_dir();
        fs::create_dir_all(&instances_dir)
            .map_err(|error| InstanceError::io("create the folder", &instances_dir, error))?;

        let folder = self.data_dir.instance_dir(&name);
        match fs::create_dir(&folder) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                return Err(InstanceError::AlreadyExists(name));
            }
            Err(error) => return Err(InstanceError::io("create the folder", &folder, error)),
        }

        let instance = Instance::new(name, config, folder);
        if let Err(error) = fill_new_instance(&instance) {
            let _ = fs::remove_dir_all(instance.folder());
            return Err(error);
        }
        Ok(instance)
    }

    pub fn load(&self, name: &InstanceName) -> Result<Instance, InstanceError> {
        let folder = self.data_dir.instance_dir(name);
        let path = config_path(&folder);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound && !folder.is_dir() => {
                return Err(InstanceError::NotFound(name.clone()));
            }
            Err(error) => return Err(InstanceError::io("read", path, error)),
        };
        let mut config =
            InstanceConfig::from_text(&text).map_err(|reason| InstanceError::BadConfig {
                name: name.clone(),
                reason,
            })?;
        follow_linked_version(&mut config, &path);
        Ok(Instance::new(name.clone(), config, folder))
    }

    pub fn list(&self) -> Result<Vec<Result<Instance, InstanceError>>, InstanceError> {
        let instances_dir = self.data_dir.instances_dir();
        let entries = match fs::read_dir(&instances_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(InstanceError::io("read", &instances_dir, error)),
        };
        let mut folders = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| InstanceError::io("read", &instances_dir, error))?;
            if entry.path().is_dir() {
                folders.push(entry.path());
            }
        }
        folders.sort();
        Ok(folders
            .iter()
            .map(|folder| self.load_folder(folder))
            .collect())
    }

    pub fn free_name(&self, base: &InstanceName) -> Option<InstanceName> {
        let is_free = |name: &InstanceName| !self.data_dir.instance_dir(name).exists();
        if is_free(base) {
            return Some(base.clone());
        }
        (2_u32..)
            .map(|number| format!("{base} ({number})").parse::<InstanceName>())
            .map_while(Result::ok)
            .find(is_free)
    }

    pub fn rename(&self, from: &InstanceName, to: InstanceName) -> Result<Instance, InstanceError> {
        let instance = self.load(from)?;
        let target = self.data_dir.instance_dir(&to);
        if target.exists() {
            return Err(InstanceError::AlreadyExists(to));
        }
        fs::rename(instance.folder(), &target)
            .map_err(|error| InstanceError::io("rename", instance.folder(), error))?;
        Ok(Instance::new(to, instance.config().clone(), target))
    }

    pub fn delete(&self, name: &InstanceName) -> Result<(), InstanceError> {
        let folder = self.data_dir.instance_dir(name);
        if !folder.is_dir() {
            return Err(InstanceError::NotFound(name.clone()));
        }
        fs::remove_dir_all(&folder).map_err(|error| InstanceError::io("delete", folder, error))
    }

    fn load_folder(&self, folder: &Path) -> Result<Instance, InstanceError> {
        let folder_name = folder.file_name().unwrap_or_default().to_string_lossy();
        let name =
            folder_name
                .parse::<InstanceName>()
                .map_err(|reason| InstanceError::BadFolderName {
                    folder: folder.to_path_buf(),
                    reason,
                })?;
        self.load(&name)
    }
}

fn follow_linked_version(config: &mut InstanceConfig, config_path: &Path) {
    let Some(folder) = &config.game_folder else {
        return;
    };
    let Ok(current) = recognize_install(folder) else {
        return;
    };
    if current.version != config.version {
        config.version = current.version;
        let _ = fs::write(config_path, config.to_text());
    }
}

fn fill_new_instance(instance: &Instance) -> Result<(), InstanceError> {
    let own_game_dir = (!instance.is_linked()).then(|| instance.game_dir());
    for dir in own_game_dir.into_iter().chain([instance.save_dir()]) {
        fs::create_dir(&dir)
            .map_err(|error| InstanceError::io("create the folder", &dir, error))?;
    }
    let path = instance.config_path();
    fs::write(&path, instance.config().to_text())
        .map_err(|error| InstanceError::io("write", path, error))
}
