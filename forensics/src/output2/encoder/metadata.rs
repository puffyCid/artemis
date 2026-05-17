use crate::output2::context::ArtifactContext;
use serde_json::{Value, json};

pub(crate) fn append_metadata(record: &mut Value, context: &ArtifactContext) {
    if let Value::Object(fields) = record {
        fields.insert(
            String::from("collection_metadata"),
            json!({
                "endpoint_id": context.endpoint_id,
                "id": context.collection_id,
                "collection_name": context.collection_name,
                "uuid": context.metadata_uuid,
                "artifact_name": context.artifact_name,
                "complete_time": context.complete_time,
                "start_time": context.start_time,
                "hostname": context.system.hostname,
                "os_version": context.system.os_version,
                "platform": context.system.platform,
                "kernel_version": context.system.kernel_version,
                "load_performance": context.system.performance,
                "artemis_version": context.system.artemis_version,
                "rust_version": context.system.rust_version,
                "build_date": context.system.build_date,
                "interfaces": context.system.interfaces,
            }),
        );
    }
}
