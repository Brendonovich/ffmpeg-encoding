use ffmpeg::{codec, format::Pixel};
use ffmpeg_encoding::{CodecContextExt, HwDevice};
use ffmpeg_sys::AVHWDeviceType;

const PATH: &str = r#"./assets/display.mp4"#;

// no hw acceleration
// fn main() -> anyhow::Result<()> {
//     let mut input_ctx = ffmpeg::format::input(PATH).unwrap();
//     let input_stream = input_ctx
//         .streams()
//         .best(ffmpeg::media::Type::Video)
//         .unwrap();
//     let input_stream_index = input_stream.index();

//     let decoder_codec = codec::decoder::find(codec::Id::H264).unwrap();

//     let mut context = codec::context::Context::new_with_codec(decoder_codec);
//     context.set_parameters(input_stream.parameters()).unwrap();

//     let mut decoder = context.decoder().video().unwrap();

//     let mut frame = ffmpeg::frame::Video::empty();

//     for (stream, packet) in input_ctx.packets() {
//         if stream.index() == input_stream_index {
//             decoder.send_packet(&packet).unwrap();

//             while decoder.receive_frame(&mut frame).is_ok() {
//                 // dbg!(frame.data(0).len());
//             }
//         }
//     }

//     while decoder.receive_frame(&mut frame).is_ok() {
//         // dbg!(frame.data(0).len());
//     }

//     Ok(())
// }

// hw acceleration
fn main() -> anyhow::Result<()> {
    let mut input_ctx = ffmpeg::format::input(PATH).unwrap();
    let input_stream = input_ctx
        .streams()
        .best(ffmpeg::media::Type::Video)
        .unwrap();
    let input_stream_index = input_stream.index();

    let decoder_codec = codec::decoder::find(codec::Id::H264).unwrap();

    let mut context = codec::context::Context::new_with_codec(decoder_codec);
    context.set_parameters(input_stream.parameters()).unwrap();

    let mut decoder = context.decoder().video().unwrap();

    let _hw_device = decoder
        .try_use_hw_device(
            AVHWDeviceType::AV_HWDEVICE_TYPE_VIDEOTOOLBOX,
            Pixel::YUV420P,
        )
        .ok();

    let mut frame = ffmpeg::frame::Video::empty();

    for (stream, packet) in input_ctx.packets() {
        if stream.index() == input_stream_index {
            decoder.send_packet(&packet).unwrap();

            while decoder.receive_frame(&mut frame).is_ok() {}
        }
    }

    while decoder.receive_frame(&mut frame).is_ok() {}

    Ok(())
}
