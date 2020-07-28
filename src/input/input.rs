use super::hal_definition::HalDefinition;

pub struct Input {
    pub svd: patch_svd::Svd,
}

impl Input {
    pub fn read(hal_config_path: String) -> Input {
        //Read hal_configuration into structure
        let hal_definition = HalDefinition::read(&hal_config_path);

        let svd_patch_path = patch_svd::get_parent_directory(&hal_config_path)
            + "/"
            + &hal_definition.svd_patch_path;
        //Take svd_path from hal_configuration
        Input {
            svd: patch_svd::read_svd_config(&svd_patch_path),
        }
    }
}
