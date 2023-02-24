use std::collections::HashSet;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct User {
    permissions: Permissions,
    subscriptions: HashSet<String>,
}

impl User {
    pub fn new() -> Self {
        Self {
            permissions: Permissions::default(),
            subscriptions: HashSet::new(),
        }
    }

    pub fn permissions(&self) -> &Permissions {
        &self.permissions
    }

    pub fn permissions_mut(&mut self) -> &mut Permissions {
        &mut self.permissions
    }

    pub fn subscriptions(&self) -> &HashSet<String> {
        &self.subscriptions
    }

    pub fn subscriptions_mut(&mut self) -> &mut HashSet<String> {
        &mut self.subscriptions
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Hash)]
pub struct Permissions {
    pub edit_kb: bool,
    pub receive_service_notifications: bool,
    pub receive_feedback: bool,
    pub admin: bool,
    pub manage_events: bool,
    pub send_global_notifications: bool,
}

impl Permissions {
    pub fn all() -> Self {
        Self {
            edit_kb: true,
            receive_service_notifications: true,
            receive_feedback: true,
            admin: true,
            manage_events: true,
            send_global_notifications: true,
        }
    }
}
