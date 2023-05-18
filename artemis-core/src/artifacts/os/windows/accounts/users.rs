use super::error::AccountError;
use crate::{
    artifacts::os::windows::{
        registry::helper::get_registry_keys, securitydescriptor::sid::grab_sid,
    },
    utils::{
        encoding::base64_decode_standard,
        nom_helper::{
            nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian,
        },
        regex_options::create_regex,
        time::filetime_to_unixepoch,
    },
};
use log::error;
use nom::bytes::complete::{take, take_until};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub(crate) struct UserInfo {
    last_logon: i64,
    password_last_set: i64,
    account_expires: i64,
    last_password_failure: i64,
    relative_id: u32,
    primary_group_id: u32,
    user_account_control_flags: Vec<UacFlags>,
    country_code: u16,
    code_page: u16,
    number_password_failures: u16,
    number_logons: u16,
    username: String,
    sid: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) enum UacFlags {
    AccountDisabled,
    HomeDirectoryRequired,
    PasswordNotRequired,
    TempDuplicateAccount,
    NormalAccount,
    MNSLogonAccount,
    InterdomainTrustAccount,
    WorkstationTrustAccount,
    ServerTrustAccount,
    DontExpirePassword,
    AccountAutoLocked,
    EncryptedTextPasswordAllowed,
    SmartcardRequired,
    TrustedForDelegation,
    NotDelegated,
    UseDESKeyOnly,
    DontRequirePreauth,
    PasswordExpired,
    TrustedToAuthenticateForDelegation,
    NoAuthDataRequired,
    PartialSecretsAccount,
    UseAESKeys,
}

impl UserInfo {
    /// Parse user account info
    pub(crate) fn parse_user_info(drive: &char) -> Result<Vec<UserInfo>, AccountError> {
        // Account info could be found in multiple Registry files, currently only focusing on SAM
        let path = format!("{drive}:\\Windows\\System32\\config\\SAM");
        let reg = create_regex("").unwrap(); // Always valid
        let start_path = "";
        let reg_result = get_registry_keys(start_path, &reg, &path);
        let reg_data = match reg_result {
            Ok(result) => result,
            Err(err) => {
                error!("[accounts] Could not get user info from registry: {err:?}");
                return Err(AccountError::GetUserInfo);
            }
        };

        let mut user_rids: HashMap<String, String> = HashMap::new();
        let mut user_info: HashMap<String, String> = HashMap::new();
        let mut sid_info: HashMap<String, String> = HashMap::new();
        // Look for account data under the Users key
        for path in reg_data {
            if !path.path.contains("Account\\Users") {
                continue;
            }

            if path.path.contains("Names\\") {
                for value in path.values {
                    user_rids.insert(value.data_type.clone(), path.name.clone());
                }
                continue;
            } else if path.path.contains("\\Users\\0") {
                for value in path.values {
                    if value.value == "F" {
                        user_info.insert(path.name.clone(), value.data.clone());
                    } else if value.value == "V" {
                        sid_info.insert(path.name.clone(), value.data.clone());
                    }
                }
            }
        }

        let mut users: Vec<UserInfo> = Vec::new();
        for (key, value) in user_rids {
            let rid_result = key.parse::<u32>();
            let rid = match rid_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[accounts] Could not parse RID {key} for user: {err:?}");
                    continue;
                }
            };

            // Loop through user info in the "F" key
            for (user_key, user_value) in &user_info {
                if !user_key.contains(&format!("{rid:X}")) {
                    continue;
                }

                let decode_results = base64_decode_standard(user_value);
                let user_data = match decode_results {
                    Ok(results) => results,
                    Err(err) => {
                        error!("[accounts] Could not base64 decode user data: {err:?}");
                        continue;
                    }
                };

                let info_result = UserInfo::parse_user_data(&user_data);
                let (_, mut info) = match info_result {
                    Ok(result) => result,
                    Err(_err) => {
                        error!("[accounts] Could not parse account info for {value}");
                        continue;
                    }
                };

                info.username = value.clone();

                // Loop through user info in the "V" key
                for (key_info, value_info) in &sid_info {
                    if !key_info.contains(&format!("{rid:X}")) {
                        continue;
                    }
                    let decode_results = base64_decode_standard(value_info);
                    let info_data = match decode_results {
                        Ok(results) => results,
                        Err(err) => {
                            error!("[accounts] Could not base64 decode info data: {err:?}");
                            continue;
                        }
                    };
                    let info_result = UserInfo::get_sid(&info_data);
                    match info_result {
                        Ok((_, result)) => info.sid = result,
                        Err(_err) => {
                            continue;
                        }
                    };
                }

                users.push(info);
            }
        }
        Ok(users)
    }

    /// Parse the account data
    fn parse_user_data(data: &[u8]) -> nom::IResult<&[u8], UserInfo> {
        let (input, _major_version) = nom_unsigned_two_bytes(data, Endian::Le)?;
        let (input, _minor_version) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, _extended_flags) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, _extended_size) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let (input, last_logon) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, _unknown_last_logoff) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, password_last_set) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, account_expires) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, last_password_failure) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let (input, relative_id) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, primary_group_id) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, account_control_flags) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, country_code) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, code_page) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, number_password_failures) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, number_logons) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let user = UserInfo {
            last_logon: filetime_to_unixepoch(&last_logon),
            password_last_set: filetime_to_unixepoch(&password_last_set),
            account_expires: filetime_to_unixepoch(&account_expires),
            last_password_failure: filetime_to_unixepoch(&last_password_failure),
            relative_id,
            primary_group_id,
            user_account_control_flags: UserInfo::get_flags(&account_control_flags),
            country_code,
            code_page,
            number_password_failures,
            number_logons,
            username: String::new(),
            sid: String::new(),
        };

        Ok((input, user))
    }

    /// Determine the account flags
    fn get_flags(account_control: &u32) -> Vec<UacFlags> {
        let disabled = 0x1;
        let home_dir = 0x2;
        let no_pass = 0x4;
        let temp_dupe = 0x8;
        let normal = 0x10;
        let user_mns = 0x20;
        let interdomain = 0x40;
        let workstation = 0x80;
        let server_trust = 0x100;
        let dont_expire = 0x2000;
        let auto_lock = 0x400;
        let text_pass = 0x800;
        let smartcard = 0x1000;
        let trusted_delegate = 0x2000;
        let not_delegate = 0x4000;
        let des_key = 0x8000;
        let dont_require_preauth = 0x10000;
        let pass_expired = 0x20000;
        let trusted_to_auth = 0x40000;
        let no_auth_data = 0x80000;
        let partial_secrets = 0x100000;
        let aes_keys = 0x200000;

        let mut flags = Vec::new();
        if (account_control & disabled) == disabled {
            flags.push(UacFlags::AccountDisabled);
        }
        if (account_control & home_dir) == home_dir {
            flags.push(UacFlags::HomeDirectoryRequired);
        }
        if (account_control & no_pass) == no_pass {
            flags.push(UacFlags::PasswordNotRequired);
        }
        if (account_control & temp_dupe) == temp_dupe {
            flags.push(UacFlags::TempDuplicateAccount);
        }
        if (account_control & normal) == normal {
            flags.push(UacFlags::NormalAccount);
        }
        if (account_control & user_mns) == user_mns {
            flags.push(UacFlags::MNSLogonAccount);
        }
        if (account_control & interdomain) == interdomain {
            flags.push(UacFlags::InterdomainTrustAccount);
        }
        if (account_control & workstation) == workstation {
            flags.push(UacFlags::WorkstationTrustAccount);
        }
        if (account_control & server_trust) == server_trust {
            flags.push(UacFlags::ServerTrustAccount);
        }
        if (account_control & dont_expire) == dont_expire {
            flags.push(UacFlags::DontExpirePassword);
        }
        if (account_control & auto_lock) == auto_lock {
            flags.push(UacFlags::AccountAutoLocked);
        }
        if (account_control & text_pass) == text_pass {
            flags.push(UacFlags::EncryptedTextPasswordAllowed);
        }
        if (account_control & smartcard) == smartcard {
            flags.push(UacFlags::SmartcardRequired);
        }
        if (account_control & trusted_delegate) == trusted_delegate {
            flags.push(UacFlags::TrustedForDelegation);
        }
        if (account_control & not_delegate) == not_delegate {
            flags.push(UacFlags::NotDelegated);
        }
        if (account_control & des_key) == des_key {
            flags.push(UacFlags::UseDESKeyOnly);
        }
        if (account_control & dont_require_preauth) == dont_require_preauth {
            flags.push(UacFlags::DontRequirePreauth);
        }
        if (account_control & pass_expired) == pass_expired {
            flags.push(UacFlags::PasswordExpired);
        }
        if (account_control & trusted_to_auth) == trusted_to_auth {
            flags.push(UacFlags::TrustedToAuthenticateForDelegation);
        }
        if (account_control & no_auth_data) == no_auth_data {
            flags.push(UacFlags::NoAuthDataRequired);
        }
        if (account_control & partial_secrets) == partial_secrets {
            flags.push(UacFlags::PartialSecretsAccount);
        }
        if (account_control & aes_keys) == aes_keys {
            flags.push(UacFlags::UseAESKeys);
        }
        flags
    }

    /// Parse the SID in the SAM data by scanning for start of SID
    fn get_sid(data: &[u8]) -> nom::IResult<&[u8], String> {
        let sid_start = [1, 5, 0, 0, 0, 0, 0];
        let (input, _) = take_until(sid_start.as_slice())(data)?;
        let sid_size: u8 = 28;
        let (_, sid_data) = take(sid_size)(input)?;

        grab_sid(sid_data)
    }
}

#[cfg(test)]
mod tests {
    use super::UserInfo;
    use crate::artifacts::os::windows::accounts::users::UacFlags;

    #[test]
    fn test_parser_user_info() {
        let test_path = 'C';
        let results = UserInfo::parse_user_info(&test_path).unwrap();
        assert!(results.len() > 2);
    }

    #[test]
    fn test_get_sid() {
        let test = [
            1, 5, 0, 0, 0, 0, 0, 5, 21, 0, 0, 0, 62, 194, 90, 64, 212, 219, 66, 139, 19, 23, 51,
            56, 248, 1, 0, 0,
        ];
        let (_, sid) = UserInfo::get_sid(&test).unwrap();
        assert_eq!(sid, "S-1-5-21-1079689790-2336414676-942872339-504");
    }

    #[test]
    fn test_get_flags() {
        let test = 1;
        let flags = UserInfo::get_flags(&test);
        assert_eq!(flags[0], UacFlags::AccountDisabled);
    }

    #[test]
    fn test_parse_user_data() {
        let test = [
            3, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 35, 156, 44,
            63, 179, 135, 213, 1, 255, 255, 255, 255, 255, 255, 255, 127, 0, 0, 0, 0, 0, 0, 0, 0,
            248, 1, 0, 0, 1, 2, 0, 0, 17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0,
        ];
        let (_, results) = UserInfo::parse_user_data(&test).unwrap();
        assert_eq!(results.account_expires, 910692730085);
        assert_eq!(results.last_logon, -11644473600);
        assert_eq!(results.password_last_set, 1571623200);
        assert_eq!(results.last_password_failure, -11644473600);
        assert_eq!(results.relative_id, 504);
        assert_eq!(results.primary_group_id, 513);
        assert_eq!(
            results.user_account_control_flags,
            vec![UacFlags::AccountDisabled, UacFlags::NormalAccount]
        );
        assert_eq!(results.country_code, 0);
        assert_eq!(results.code_page, 0);
        assert_eq!(results.number_password_failures, 0);
        assert_eq!(results.number_logons, 0);
    }
}
