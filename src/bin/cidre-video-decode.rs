use cidre::{av, ns};

const PATH: &str = r#"./assets/display.mp4"#;

#[tokio::main]
async fn main() {
    let asset = av::UrlAsset::with_url(&ns::Url::with_fs_path_str(PATH, false), None).unwrap();

    let mut reader = av::AssetReader::with_asset(&asset).unwrap();

    let tracks = asset
        .load_tracks_with_media_type(av::MediaType::video())
        .await
        .unwrap();

    let track = &tracks[0];

    let mut reader_track_output = av::AssetReaderTrackOutput::with_track(
        track,
        Some(&ns::Dictionary::with_keys_values(
            &[ns::String::with_str("PixelFormatType").as_ref()],
            &[ns::Number::with_u32(875704438).as_id_ref()],
        )),
    )
    .unwrap();

    reader_track_output.set_always_copies_sample_data(true);

    reader.add_output(&reader_track_output).unwrap();

    reader.start_reading();

    while let Ok(Some(sample_buf)) = reader_track_output.copy_next_sample_buf() {
        // if let Some(image_buf) = sample_buf.image_buf() {
        // dbg!(image_buf.color_space());
        // }
    }
}
