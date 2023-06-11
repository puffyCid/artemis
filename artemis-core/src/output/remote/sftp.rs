use super::error::RemoteError;
use crate::utils::{
    artemis_toml::Output, compression::compress_gzip_data, encoding::base64_decode_standard,
};
use log::error;
use ssh2::Session;
use std::{io::Write, net::TcpStream, path::Path};

/// Upload data to SFTP server using password or SSH key
pub(crate) fn sftp_output(
    data: &[u8],
    output: &Output,
    output_name: &str,
) -> Result<(), RemoteError> {
    let sess_result = Session::new();
    let mut sess = match sess_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Could not create SFTP session: {err:?}");
            return Err(RemoteError::SftpSession);
        }
    };
    let timeout_milliseconds = 6000000;
    // Set timeout to 100 minutes. May want to set this to zero (0) for unlimited?
    sess.set_timeout(timeout_milliseconds);
    let sftp_url = if let Some(url) = &output.url {
        url
    } else {
        return Err(RemoteError::RemoteUrl);
    };

    let sftp_port = if let Some(port) = &output.port {
        port
    } else {
        return Err(RemoteError::RemotePort);
    };

    let tcp_result = TcpStream::connect(format!("{sftp_url}:{sftp_port}"));
    let tcp = match tcp_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Failed to create SFTP TCP Connect Stream: {err:?}");
            return Err(RemoteError::TcpConnect);
        }
    };

    sess.set_tcp_stream(tcp);
    let handshake_result = sess.handshake();
    match handshake_result {
        Ok(_) => {}
        Err(err) => {
            println!("[artemis-core] Failed to establish SFTP handshake: {err:?}");
            return Err(RemoteError::SftpHandshake);
        }
    }

    let sftp_username = if let Some(username) = &output.username {
        username
    } else {
        return Err(RemoteError::SftpUsername);
    };

    if let Some(password) = &output.password {
        let auth_result = sess.userauth_password(sftp_username, password);
        match auth_result {
            Ok(_) => {}
            Err(err) => {
                error!("[artemis-core] Failed to authenticate with SFTP password: {err:?}");
                return Err(RemoteError::SftpPassword);
            }
        }
    } else if let Some(key) = &output.api_key {
        let data = base64_decode_standard(key).unwrap();
        let sftp_key = String::from_utf8(data).unwrap();
        let auth_result = sess.userauth_pubkey_memory(sftp_username, None, &sftp_key, None);
        match auth_result {
            Ok(_) => {}
            Err(err) => {
                error!("[artemis-core] Failed to authenticate with SFTP password: {err:?}");
                return Err(RemoteError::SftpPassword);
            }
        }
    } else {
        return Err(RemoteError::SftpNoAuth);
    }

    let mut sftp_output = format!("{}/{output_name}.{}", output.directory, output.format);
    let output_data = if output.compress {
        sftp_output = format!("{sftp_output}.gz");
        compress_gzip_data(data).unwrap()
    } else {
        data.to_vec()
    };

    let sftp_file_result = sess.sftp();
    let sftp_file = match sftp_file_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Failed to open SFTP channel: {err:?}");
            return Err(RemoteError::SftpChannel);
        }
    };
    let remote_file_result = sftp_file.create(Path::new(&sftp_output));

    let mut remote_file = match remote_file_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Failed to create SFTP file: {err:?}");
            return Err(RemoteError::CreateFile);
        }
    };

    let write_result = remote_file.write_all(&output_data);
    match write_result {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Failed to write SFTP file: {err:?}");
            return Err(RemoteError::FileWrite);
        }
    }
    let close_result = remote_file.close();
    match close_result {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Failed to close SFTP file write: {err:?}");
            return Err(RemoteError::FileClose);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{output::remote::sftp::sftp_output, utils::artemis_toml::Output};

    fn output_options(
        name: &str,
        output: &str,
        directory: &str,
        compress: bool,
        port: u16,
    ) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            url: Some(String::from("127.0.0.1")),
            port: Some(port),
            api_key: Some(String::new()),
            username: Some(String::from("foo")),
            password: Some(String::from("pass")),
            generic_keys: Some(Vec::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        }
    }

    #[test]
    fn test_upload_sftp() {
        let output = output_options("sftp_upload_test", "sftp", "tmp", false, 2222);

        let test = "A rust program";
        let name = "output";
        let result = sftp_output(test.as_bytes(), &output, name).unwrap();
        assert_eq!(result, ());
    }

    #[test]
    fn test_upload_sftp_compress() {
        let output = output_options("sftp_upload_test", "sftp", "tmp", false, 2222);

        let test = "A rust program";
        let name = "output";
        let result = sftp_output(test.as_bytes(), &output, name).unwrap();
        assert_eq!(result, ());
    }

    #[test]
    #[should_panic(expected = "TcpConnect")]
    fn test_bad_upload_sftp() {
        let output = Output {
            name: String::from("test_output"),
            directory: String::from("upload"),
            format: String::from("sftp"),
            compress: false,
            url: Some(String::from("127.0.0.1")),
            port: Some(2223),
            api_key: Some(String::new()),
            username: Some(String::from("foo")),
            password: Some(String::from("pass")),
            generic_keys: Some(Vec::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        };

        let test = "A rust program";
        let name = "output";
        let result = sftp_output(test.as_bytes(), &output, name).unwrap();
        assert_eq!(result, ());
    }
}
