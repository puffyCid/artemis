use super::schema::{registration::RegistrationInfo, triggers::Triggers};

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

struct Settings {
    allow_start_on_demand: Option<bool>,
    restart_on_failure: Option<RestartType>,
    multiple_instances_policy: Option<String>,
    disallow_start_if_on_battiers: Option<bool>,
    stop_if_going_on_batteries: Option<bool>,
    allow_hard_terminate: Option<bool>,
    start_when_available: Option<bool>,
    newtork_profile_name: Option<String>,
    run_only_if_network_available: Option<bool>,
    wake_to_run: Option<bool>,
    enabled: Option<bool>,
    hidden: Option<bool>,
    delete_expired_tasks_after: Option<String>,
    idle_settings: Option<IdleSettings>,
    network_settings: Option<NetworkSettings>,
    execution_time_limit: Option<String>,
    priority: Option<u8>,
    run_only_if_idle: Option<bool>,
    use_unified_scheduling_engine: Option<bool>,
    disallow_start_on_remote_app_session: Option<bool>,
    maintence_settings: Option<MaintenceSettings>,
    volatile: Option<bool>,
}

struct RestartType {
    interval: String,
    count: u16,
}

struct IdleSettings {
    duration: Option<String>,
    wait_timeout: Option<String>,
    stop_on_idle_end: Option<bool>,
    restart_on_idle: Option<bool>,
}

struct NetworkSettings {
    name: Option<String>,
    id: Option<String>,
}

struct MaintenceSettings {
    period: String,
    deadline: Option<String>,
    exclusive: Option<bool>,
}

struct Principals {
    user_id: Option<String>,
    logon_type: Option<String>,
    group_id: Option<String>,
    display_nme: Option<String>,
    run_level: Option<String>,
    process_token_sid_type: Option<String>,
    required_privileges: Option<Vec<String>>,
    id_attribute: Option<String>,
}

struct Actions {
    exec: Option<ExecType>,
    com_handler: Option<ComHandlerType>,
    send_email: Option<SendEmail>,
    show_message: Option<Message>,
}

struct ExecType {
    command: String,
    arguments: Option<String>,
    working_directory: Option<String>,
}

struct ComHandlerType {
    class_id: String,
    data: Option<String>,
}

struct SendEmail {
    server: Option<String>,
    subject: Option<String>,
    to: Option<String>,
    cc: Option<String>,
    bcc: Option<String>,
    reply_to: Option<String>,
    from: String,
    header_fields: Vec<String>,
    body: Option<String>,
    attachment: Option<String>,
}

struct Message {
    title: Option<String>,
    body: String,
}
