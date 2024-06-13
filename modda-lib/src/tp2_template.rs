
use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use handlebars::Handlebars;
use serde_json::json;

use std::io::Write;

use crate::canon_path::CanonPath;
use crate::module::gen_mod::GeneratedMod;

const TP2_TEMPLATE: &str ="
/*
 * TP2 generated by modda
 * {{date}}
*/
BACKUP ~weidu_external/backup/{{mod_name}}~
AUTHOR ~generated by modda~
{{description}}
BEGIN ~{{component_name}}~
DESIGNATED ~{{index}}~

COPY ~{{mod_name}}/data~ ~override~

";

pub fn generate_tp2(gen: &GeneratedMod, date: DateTime<Utc>) -> Result<String> {
    let registry = Handlebars::new();

    let comp_name = match &gen.component.name {
        None => gen.gen_mod.to_string(),
        Some(s) if s.is_empty() => gen.gen_mod.to_string(),
        Some(name) => name.to_owned(),
    };
    let result = registry.render_template(
        TP2_TEMPLATE,
        &json!({
            "date": date.to_string(),
            "mod_name": &gen.gen_mod,
            "component_name": comp_name,
            "index": gen.component.index,
            "description": match &gen.description {
                Some(desc) => format!("\n// {desc}"),
                None => "".to_string(),
            },
        })
    )?;
    Ok(result)
}

pub fn create_tp2(gen: &GeneratedMod, target: &CanonPath) -> Result<()> {
    let content = match generate_tp2(gen, Utc::now()) {
        Err(err) => bail!("Could not generate tp2 file from template\n  {}", err),
        Ok(content) => content,
    };
    let tp2_path = target.join(format!("{}.tp2", gen.gen_mod))?;
    let file = std::fs::OpenOptions::new()
                                            .write(true)
                                            .create_new(true)
                                            .open(tp2_path);
    let mut file = match file {
        Err(err) => bail!("Could not create generated tp2 file {}\n  {}", gen.gen_mod, err),
        Ok(file) => file,
    };
    if let Err(err) = write!(file, "{}", content) {
        bail!("Could not write content to generated tp2 file {}\n  {}", gen.gen_mod, err);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};

    use crate::lowercase::lwc;
    use crate::module::file_module_origin::FileModuleOrigin;
    use crate::module::gen_mod::{GenModComponent, GeneratedMod};
    use crate::tp2_template::generate_tp2;


    #[test]
    fn generate_tp2_without_description() {
        let gen_mod = GeneratedMod {
            gen_mod: lwc!("ccc"),
            files: vec![
                FileModuleOrigin::Local { local: "my_subdir".to_string(), glob: None },
            ],
            description: None,
            component: GenModComponent { index: 0, name: Some("my component".to_string()) },
            post_install: None,
            ignore_warnings: true,
            allow_overwrite: true,
            disabled_if: None
        };
        let date_time = DateTime::from_naive_utc_and_offset(
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2024, 05, 18).unwrap(),
                NaiveTime::from_hms_opt(12, 13, 14).unwrap(),
            ),
            Utc
        );
        assert_eq!(
            generate_tp2(&gen_mod,date_time).unwrap(),
            r#"
/*
 * TP2 generated by modda
 * 2024-05-18 12:13:14 UTC
*/
BACKUP ~weidu_external/backup/ccc~
AUTHOR ~generated by modda~

BEGIN ~my component~
DESIGNATED ~0~

COPY ~ccc/data~ ~override~

"#
        )
    }

    #[test]
    fn generate_tp2_with_description() {
        let gen_mod = GeneratedMod {
            gen_mod: lwc!("ccc"),
            files: vec![
                FileModuleOrigin::Local { local: "my_subdir".to_string(), glob: None },
            ],
            description: Some("Very detailed description".to_string()),
            component: GenModComponent { index: 0, name: Some("my component".to_string()) },
            post_install: None,
            ignore_warnings: true,
            allow_overwrite: true,
            disabled_if: None,
        };
        let date_time = DateTime::from_naive_utc_and_offset(
            NaiveDateTime::new(
                NaiveDate::from_ymd_opt(2024, 05, 18).unwrap(),
                NaiveTime::from_hms_opt(12, 13, 14).unwrap(),
            ),
            Utc
        );
        assert_eq!(
            generate_tp2(&gen_mod,date_time).unwrap(),
            r#"
/*
 * TP2 generated by modda
 * 2024-05-18 12:13:14 UTC
*/
BACKUP ~weidu_external/backup/ccc~
AUTHOR ~generated by modda~

// Very detailed description
BEGIN ~my component~
DESIGNATED ~0~

COPY ~ccc/data~ ~override~

"#
        )
    }
}
