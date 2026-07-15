use serde::Serialize;
use std::process::{Command, ExitStatus};

const PMSET_PATH: &str = "/usr/bin/pmset";
const OSASCRIPT_PATH: &str = "/usr/bin/osascript";

const ENABLE_DISABLED_SLEEP_SCRIPT: &str = r#"try
    do shell script "/usr/bin/pmset -a disablesleep 1" with administrator privileges
    return "applied"
on error errorMessage number errorNumber
    if errorNumber is -128 then
        return "cancelled"
    end if
    error errorMessage number errorNumber
end try"#;

const DISABLE_DISABLED_SLEEP_SCRIPT: &str = r#"try
    do shell script "/usr/bin/pmset -a disablesleep 0" with administrator privileges
    return "applied"
on error errorMessage number errorNumber
    if errorNumber is -128 then
        return "cancelled"
    end if
    error errorMessage number errorNumber
end try"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SleepStatus {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeOutcome {
    Applied,
    Cancelled,
}

pub fn read_sleep_status() -> Result<SleepStatus, String> {
    let output = Command::new(PMSET_PATH)
        .arg("-g")
        .output()
        .map_err(|error| format!("pmsetを起動できません: {error}"))?;

    if !output.status.success() {
        return Err(command_failure("pmset -g", output.status, &output.stderr));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("pmsetの出力がUTF-8ではありません: {error}"))?;
    parse_pmset_output(&stdout)
}

pub fn set_sleep_disabled(disabled: bool) -> Result<ChangeOutcome, String> {
    let script = script_for(disabled);
    let output = Command::new(OSASCRIPT_PATH)
        .args(["-e", script])
        .output()
        .map_err(|error| format!("osascriptを起動できません: {error}"))?;

    classify_osascript_output(output.status, &output.stdout, &output.stderr)
}

fn script_for(disabled: bool) -> &'static str {
    if disabled {
        ENABLE_DISABLED_SLEEP_SCRIPT
    } else {
        DISABLE_DISABLED_SLEEP_SCRIPT
    }
}

fn parse_pmset_output(output: &str) -> Result<SleepStatus, String> {
    let value = output.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        match (fields.next(), fields.next()) {
            (Some("SleepDisabled"), Some(value)) => Some(value),
            _ => None,
        }
    });

    match value {
        Some("0") => Ok(SleepStatus::Enabled),
        Some(raw) => raw
            .parse::<i64>()
            .map(|_| SleepStatus::Disabled)
            .map_err(|_| format!("SleepDisabledの値が不正です: {raw}")),
        None => Err("pmset -gの出力にSleepDisabledがありません".to_owned()),
    }
}

fn classify_osascript_output(
    status: ExitStatus,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<ChangeOutcome, String> {
    if !status.success() {
        return Err(command_failure("osascript", status, stderr));
    }

    match String::from_utf8_lossy(stdout).trim() {
        "applied" => Ok(ChangeOutcome::Applied),
        "cancelled" => Ok(ChangeOutcome::Cancelled),
        other => Err(format!("osascriptが不明な結果を返しました: {other}")),
    }
}

fn command_failure(label: &str, status: ExitStatus, stderr: &[u8]) -> String {
    let detail = String::from_utf8_lossy(stderr).trim().to_owned();
    if detail.is_empty() {
        format!("{label}が終了コード{status}で失敗しました")
    } else {
        format!("{label}が失敗しました: {detail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn parses_zero_as_sleep_enabled() {
        assert_eq!(
            parse_pmset_output("System-wide power settings:\n SleepDisabled 0\n"),
            Ok(SleepStatus::Enabled)
        );
    }

    #[test]
    fn parses_one_as_sleep_disabled() {
        assert_eq!(
            parse_pmset_output("System-wide power settings:\nSleepDisabled 1\n"),
            Ok(SleepStatus::Disabled)
        );
    }

    #[test]
    fn parses_any_nonzero_integer_as_sleep_disabled() {
        assert_eq!(
            parse_pmset_output(" SleepDisabled    -2  \n"),
            Ok(SleepStatus::Disabled)
        );
    }

    #[test]
    fn accepts_whitespace_and_unrelated_lines() {
        assert_eq!(
            parse_pmset_output("Currently in use:\n standbydelayhigh 4200\n\tSleepDisabled\t0\t\n"),
            Ok(SleepStatus::Enabled)
        );
    }

    #[test]
    fn rejects_missing_key() {
        assert_eq!(
            parse_pmset_output("System-wide power settings:\n"),
            Err("pmset -gの出力にSleepDisabledがありません".to_owned())
        );
    }

    #[test]
    fn rejects_invalid_value() {
        assert_eq!(
            parse_pmset_output("SleepDisabled maybe\n"),
            Err("SleepDisabledの値が不正です: maybe".to_owned())
        );
    }

    #[test]
    fn selects_only_fixed_scripts() {
        assert!(script_for(true).contains("/usr/bin/pmset -a disablesleep 1"));
        assert!(script_for(false).contains("/usr/bin/pmset -a disablesleep 0"));
        assert!(script_for(true).contains("errorNumber is -128"));
        assert!(script_for(false).contains("errorNumber is -128"));
    }

    #[test]
    fn classifies_success_and_cancellation() {
        let success = Command::new("/usr/bin/true").status().unwrap();
        assert_eq!(
            classify_osascript_output(success, b"applied\n", b""),
            Ok(ChangeOutcome::Applied)
        );

        let success = Command::new("/usr/bin/true").status().unwrap();
        assert_eq!(
            classify_osascript_output(success, b"cancelled\n", b""),
            Ok(ChangeOutcome::Cancelled)
        );
    }

    #[test]
    fn classifies_command_failure_and_unknown_success() {
        let failure = Command::new("/usr/bin/false").status().unwrap();
        let error = classify_osascript_output(failure, b"", b"permission denied").unwrap_err();
        assert!(error.contains("permission denied"));

        let success = Command::new("/usr/bin/true").status().unwrap();
        assert_eq!(
            classify_osascript_output(success, b"unexpected\n", b""),
            Err("osascriptが不明な結果を返しました: unexpected".to_owned())
        );
    }
}
