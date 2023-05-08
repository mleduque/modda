
use chrono::{DateTime, Local, Duration};

use crate::lowercase::LwcString;


#[derive(Default, Debug, Clone)]
pub struct InstallTimeline {
    pub name: LwcString,
    pub start: DateTime<Local>,
    pub start_download: Option<DateTime<Local>>,
    pub downloaded: Option<DateTime<Local>>,
    pub copied: Option<DateTime<Local>>,
    pub patched: Option<DateTime<Local>>,
    pub configured: Option<DateTime<Local>>,
    pub start_install: Option<DateTime<Local>>,
    pub installed: Option<DateTime<Local>>,
}

impl InstallTimeline {
    pub fn new(name: LwcString, start: DateTime<Local>) -> Self {
        InstallTimeline {
            name,
            start,
            ..Default::default()
        }
    }

    pub fn complete(&mut self, setup: SetupTimeline) {
        self.start_download = Some(setup.start);
        self.downloaded = setup.downloaded;
        self.copied = setup.copied;
        self.patched = setup.patched;
        self.configured = setup.configured;
    }

    pub fn short(&self) -> String {
        let mut result = format!("({}) {}" , self.start.format("%H:%M:%S"), self.name);
            result +=" download: ";
        if let (Some(start), Some(end)) = (self.start_download, self.downloaded) {
            result+= &format_duration(end - start).to_string();
        } else {
            result += "-"
        }
            result +=" extract: ";
        if let (Some(downloaded), Some(copied)) = (self.downloaded, self.copied) {
            result+= &format_duration(copied - downloaded).to_string();
        } else {
            result += "-"
        }
            result +=" prepare: ";
        if let (Some(copied), Some(configured)) = (self.copied, self.configured) {
            result+= &format_duration(configured - copied).to_string();
        } else {
            result += "-"
        }
            result +=" install: ";
        if let (Some(start_install), Some(installed)) = (self.start_install, self.installed) {
            result+= &format_duration(installed - start_install).to_string();
        } else {
            result += "-"
        }
        result
    }
}

fn format_duration(duration: Duration) -> String {
    let duration_as_seconds = Duration::seconds(duration.num_seconds());
    humantime::format_duration(duration_as_seconds.to_std().unwrap()).to_string()
}

fn format_duration_2(duration: Duration) -> String {
    let minutes = duration.num_minutes();
    let minutes_string = format!("{}", minutes);
    let minutes_string = if minutes_string.len() < 2 { format!("0{}", minutes_string) } else{ minutes_string };
    let rest = duration - Duration::minutes(minutes);
    let seconds = rest.num_seconds();
    let rest = rest - Duration::seconds(seconds);
    let millis = rest.num_milliseconds();
    format!("{}min {:02}s {:02}ms", minutes_string, seconds, millis)
}

#[derive(Default, Debug, Clone)]
pub struct SetupTimeline {
    pub start: DateTime<Local>,
    pub downloaded: Option<DateTime<Local>>,
    pub copied: Option<DateTime<Local>>,
    pub patched: Option<DateTime<Local>>,
    pub replaced: Option<DateTime<Local>>,
    pub configured: Option<DateTime<Local>>,
}
