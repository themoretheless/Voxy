fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "artifacts/animal-studies".into());
    std::fs::create_dir_all(&output)?;
    for breed in voxy_model::animals::Breed::ALL {
        std::fs::write(
            format!("{output}/{}.glb", breed.id()),
            voxy_model::animals::study_glb(breed)?,
        )?;
    }
    std::fs::write(
        format!("{output}/gallery.glb"),
        voxy_model::animals::gallery_glb()?,
    )?;
    Ok(())
}
