#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub enum Filter {
    AvailableOnly {
        utilities: bool,
    },
    Catalog {
        force_craftable: bool,
        from_task: bool,
        from_npc: bool,
        from_monster: bool,
        utilities: bool,
    },
}

impl Default for Filter {
    fn default() -> Self {
        Self::Catalog {
            force_craftable: true,
            from_task: false,
            from_npc: true,
            from_monster: false,
            utilities: false,
        }
    }
}

impl Filter {
    #[must_use]
    pub const fn is_available_only(self) -> bool {
        matches!(self, Self::AvailableOnly { .. })
    }

    #[must_use]
    pub const fn utilities_allowed(self) -> bool {
        matches!(
            self,
            Self::AvailableOnly { utilities: true }
                | Self::Catalog {
                    utilities: true,
                    ..
                }
        )
    }

    #[must_use]
    pub const fn available_only() -> Self {
        Self::AvailableOnly { utilities: false }
    }

    #[must_use]
    pub const fn everything() -> Self {
        Self::Catalog {
            force_craftable: true,
            from_task: true,
            from_npc: true,
            from_monster: true,
            utilities: true,
        }
    }
}
