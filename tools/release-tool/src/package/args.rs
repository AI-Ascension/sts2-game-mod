// SPDX-License-Identifier: MIT

use crate::common::fail;

#[derive(Debug)]
pub(crate) struct Args {
    pub(crate) platform: String,
    pub(crate) payload_dir: String,
    pub(crate) output_dir: String,
    pub(crate) app_id: String,
    pub(crate) item_id: String,
    pub(crate) game_version: String,
    pub(crate) package_version: String,
    pub(crate) source_revision: String,
    pub(crate) preview_file: String,
    pub(crate) build_manifest: Option<String>,
    pub(crate) legacy: bool,
}

pub(crate) fn parse_args(arguments: &[String]) -> Result<Args, String> {
    if arguments.len() < 9 {
        return fail(
            "usage: package-platform-item.sh PLATFORM PAYLOAD OUTPUT APP_ID ITEM_ID GAME_VERSION PACKAGE_VERSION SOURCE_REVISION PREVIEW [--build-manifest PATH|--legacy-unbound]",
        );
    }
    let positional = &arguments[..9];
    let mut build_manifest = None;
    let mut legacy = false;
    let mut index = 9;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--build-manifest" => {
                if build_manifest.is_some() || legacy {
                    return fail("provenance options are mutually exclusive");
                }
                build_manifest = Some(
                    arguments
                        .get(index + 1)
                        .ok_or("--build-manifest requires a path")?
                        .clone(),
                );
                index += 2;
            }
            "--legacy-unbound" => {
                if build_manifest.is_some() || legacy {
                    return fail("provenance options are mutually exclusive");
                }
                legacy = true;
                index += 1;
            }
            _ => return fail(format!("unknown option: {}", arguments[index])),
        }
    }
    Ok(Args {
        platform: positional[0].clone(),
        payload_dir: positional[1].clone(),
        output_dir: positional[2].clone(),
        app_id: positional[3].clone(),
        item_id: positional[4].clone(),
        game_version: positional[5].clone(),
        package_version: positional[6].clone(),
        source_revision: positional[7].clone(),
        preview_file: positional[8].clone(),
        build_manifest,
        legacy,
    })
}

pub(crate) fn bounded_decimal(value: &str, maximum: u64, label: &str) -> Result<u64, String> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return fail(format!(
            "{label} must be an unsigned decimal without leading zeroes"
        ));
    }
    let number = value
        .parse::<u64>()
        .map_err(|_| format!("{label} is not a decimal integer"))?;
    if number > maximum {
        return fail(format!("{label} exceeds its unsigned bound"));
    }
    Ok(number)
}
