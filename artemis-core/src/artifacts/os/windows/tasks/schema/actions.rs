pub(crate) struct Actions {
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
