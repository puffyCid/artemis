use super::schema::{
    actions::Actions, principals::Principals, registration::RegistrationInfo, settings::Settings,
    triggers::Triggers,
};

/**
 * Structure of a XML format Schedule Task
 * Schema at: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-tsch/0d6383e4-de92-43e7-b0bb-a60cfa36379f
 */
pub(crate) struct TaskData {
    registration_info: Option<RegistrationInfo>,
    triggers: Option<Triggers>,
    settings: Option<Settings>,
    /**Raw bytes, base64 encoded */
    data: Option<String>,
    principals: Option<Principals>,
    actions: Actions,
}
