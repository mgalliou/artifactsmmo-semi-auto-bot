#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct Filter {
    pub available_only: bool,
    pub craftable: bool,
    pub from_task: bool,
    pub from_npc: bool,
    pub from_monster: bool,
    pub utilities: bool,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            available_only: false,
            craftable: true,
            from_task: true,
            from_npc: true,
            from_monster: false,
            utilities: false,
        }
    }
}

impl Filter {
    pub const fn available_only() -> Self {
        Self {
            available_only: true,
            craftable: false,
            from_task: false,
            from_npc: false,
            from_monster: false,
            utilities: false,
        }
    }

    pub const fn everything() -> Self {
        Self {
            available_only: false,
            craftable: true,
            from_task: true,
            from_npc: true,
            from_monster: true,
            utilities: true,
        }
    }
}
