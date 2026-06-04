use crate::{
    output2::{
        error::{OutputError, OutputResult},
        report::{ArtifactRunReport, hash_artifact_options},
    },
    utils::time::time_now,
};
use log::error;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, create_dir_all},
    path::PathBuf,
};

/// Determine if an artifact was recently collected
#[derive(Debug, Deserialize, Serialize)]
pub struct MarkerTracker {
    /// Path to save the marker file to
    path: PathBuf,
    /// Name of marker file
    name: String,
    /// Age in minutes
    age: u64,
}

impl MarkerTracker {
    /// Determine if we should skip parsing this artifact
    pub(crate) fn should_skip<T: Serialize>(
        &self,
        artifact_name: &str,
        artifact_options: &T,
    ) -> OutputResult<bool> {
        let hash = hash_artifact_options(artifact_options)?;
        let runs = self.read_runs()?;

        let now = time_now();
        let seconds = 60;
        let next_run = self.age.saturating_mul(seconds);

        // Loop through our marker file. If the artifact we want to parse matches
        // When we match, we skip the artifact
        let run_hit = runs.into_iter().any(|run| {
            run.name == artifact_name
                && run.artifact_options_hash == hash
                && run.last_run_epoch.saturating_add(next_run) > now
        });
        Ok(run_hit)
    }

    /// Update the `MarkerTracker` JSON file
    pub(crate) fn update_runs(&self, new_runs: &[ArtifactRunReport]) -> OutputResult<()> {
        // If we cannot read the marker file. We create a new one
        let mut runs = match self.read_runs() {
            Ok(results) => results,
            Err(err) => {
                error!("[forensics] Could not read marker file {err:?}. Overwriting it");
                Vec::new()
            }
        };
        for run in new_runs {
            if let Some(exists) = runs.iter_mut().find(|existing| {
                existing.name == run.name
                    && existing.artifact_options_hash == run.artifact_options_hash
            }) {
                *exists = run.clone();
            } else {
                runs.push(run.clone());
            }
        }

        let path = self.path.join(&self.name);

        create_dir_all(&self.path).map_err(|err| OutputError::io_path(&self.path, err))?;
        let file = File::create(&path).map_err(|err| OutputError::io_path(&path, err))?;
        serde_json::to_writer(file, &runs)?;

        Ok(())
    }

    /// Read `ArtifactRunReport` values from our marker file
    fn read_runs(&self) -> OutputResult<Vec<ArtifactRunReport>> {
        let path = self.path.join(&self.name);
        if !path.is_file() {
            return Ok(Vec::new());
        }

        let file = File::open(&path).map_err(|err| OutputError::io_path(&path, err))?;
        let runs = serde_json::from_reader(file)?;
        Ok(runs)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        output2::{
            marker::MarkerTracker,
            report::{ArtifactRunReport, hash_artifact_options},
        },
        utils::time::time_now,
    };
    use serde_json::{Value, json};
    use std::path::PathBuf;

    fn create_report(options: Value) -> ArtifactRunReport {
        ArtifactRunReport {
            name: String::from("processes"),
            artifact_options_hash: hash_artifact_options(&options).unwrap(),
            artifact_options: options,
            last_run: String::from("1970-01-01T00:00:00.000Z"),
            last_run_epoch: time_now(),
            output_count: 0,
            record_count: 1,
            output_files: Vec::new(),
            status: String::from("completed"),
        }
    }

    #[test]
    fn test_marker_read_runs() {
        let test_location =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/marker");

        let tracker = MarkerTracker {
            path: test_location,
            name: String::from("test2.json"),
            age: 300,
        };

        let runs = tracker.read_runs().unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(
            runs[0].artifact_options_hash,
            "2297a2e4d2902655a171ae9b818ce092"
        );
    }

    #[test]
    fn test_marker_update() {
        let tracker = MarkerTracker {
            path: PathBuf::from("./tmp/marker_update/"),
            name: String::from("test2.json"),
            age: 300,
        };

        let run =
            create_report(json!({"md5": true, "sha1": false, "sha256": false, "metadata": true}));
        tracker.update_runs(&[run]).unwrap();

        let run =
            create_report(json!({"md5": false, "sha1": false, "sha256": false, "metadata": true}));
        tracker.update_runs(&[run]).unwrap();

        let runs = tracker.read_runs().unwrap();
        assert_eq!(runs.len(), 2);
    }

    #[test]
    fn test_marker_skip_artifact() {
        let tracker = MarkerTracker {
            path: PathBuf::from("./tmp/marker_update/"),
            name: String::from("test3.json"),
            age: 300,
        };

        let run =
            create_report(json!({"md5": true, "sha1": false, "sha256": false, "metadata": true}));

        tracker.update_runs(&[run.clone()]).unwrap();
        assert!(
            tracker
                .should_skip("processes", &run.artifact_options)
                .unwrap()
        );

        let run =
            create_report(json!({"md5": false, "sha1": false, "sha256": false, "metadata": true}));
        assert!(
            !tracker
                .should_skip("processes", &run.artifact_options)
                .unwrap()
        );
    }
}
