use super::*;

pub(crate) fn cmd_il(
    path: PathBuf,
    format: Format,
    normalized: bool,
    no_cfg_norm: bool,
) -> Result<()> {
    let path_str = path.to_string_lossy().to_string();
    let lang = Lang::from_path(&path_str)
        .with_context(|| format!("unsupported file extension: {path_str}"))?;
    let src = std::fs::read(&path).with_context(|| format!("reading {path_str}"))?;
    let interner = Interner::new();
    // Use the region-aware entry so `<script>`/`<style>`/markup of a Vue/Svelte/HTML
    // container are each shown (single-region languages still yield exactly one Il).
    let regions = nose_frontend::lower_source_regions(FileId(0), &path_str, &src, lang, &interner);
    if regions.is_empty() {
        anyhow::bail!("no analyzable region lowered from {path_str}");
    }
    let multi = regions.len() > 1;
    for raw in regions {
        let region_lang = raw.meta.lang;
        let il = if normalized {
            let opts = nose_normalize::NormalizeOptions {
                cfg_norm: !no_cfg_norm,
                ..Default::default()
            };
            nose_normalize::normalize(&raw, &interner, &opts)
        } else {
            raw
        };
        match format {
            Format::Sexpr => {
                if multi {
                    println!("; region: {}", region_lang.name());
                }
                println!("{}", il.to_sexpr(il.root, &interner));
            }
            Format::Json => {
                println!("{}", serde_json::to_string_pretty(&il)?);
            }
        }
    }
    Ok(())
}
