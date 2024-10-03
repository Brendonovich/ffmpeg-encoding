use std::{
    io::Write,
    ptr::{null, null_mut},
};

use ffmpeg::ffi::*;
use ffmpeg_next as ffmpeg;
use scap::{
    capturer::{Capturer, Options},
    frame::Frame,
};

/*
 * Trying to replicate the following command:
 * ffmpeg -f rawvideo -pix_fmt bgra -s {}x[} -r 30 -i frames.raw -c:v libx264 -pix_fmt yuv420p output-cli.mp4
 */

fn main() {
    if !scap::is_supported() {
        println!("❌ Platform not supported");
        return;
    }

    if !scap::has_permission() {
        println!("❌ Permission not granted. Requesting permission...");
        if !scap::request_permission() {
            println!("❌ Permission denied");
            return;
        }
    }

    let fps = 30;

    let options = Options {
        fps,
        show_cursor: true,
        show_highlight: true,
        excluded_targets: None,
        output_type: scap::frame::FrameType::BGRAFrame,
        output_resolution: scap::capturer::Resolution::_720p,
        ..Default::default()
    };

    let mut recorder = Capturer::new(options);

    let [width, height] = recorder.get_output_frame_size();

    recorder.start_capture();

    let mut frames = vec![];

    let mut start_time: u64 = 0;
    for i in 0..100 {
        let frame = recorder.get_next_frame().expect("Error");

        match frame {
            Frame::BGRA(frame) => {
                if start_time == 0 {
                    start_time = frame.display_time;
                }
                println!(
                    "Recieved BGRA frame {} of width {} and height {} and time {}",
                    i,
                    frame.width,
                    frame.height,
                    frame.display_time - start_time
                );

                frames.push(frame.data);
            }
            _ => continue,
        }
    }

    recorder.stop_capture();

    println!("Collecting frames to bytes");
    let buf = frames.into_iter().flatten().collect::<Vec<u8>>();

    println!("Writing frames to frames.raw...");
    std::fs::File::create("frames.raw")
        .unwrap()
        .write_all(&buf)
        .unwrap();

    let source_format = AVPixelFormat::AV_PIX_FMT_BGRA;

    unsafe {
        let mut output = null_mut();
        let filename = c"output.mp4".as_ptr();
        avformat_alloc_output_context2(&mut output, null(), null(), filename);

        let format = (*output).oformat;

        let codec = avcodec_find_encoder(AVCodecID::AV_CODEC_ID_H264);

        let stream = avformat_new_stream(output, null());
        (*stream).id = ((*output).nb_streams - 1) as i32;

        let mut encoder = avcodec_alloc_context3(codec);

        (*encoder).codec_id = (*format).video_codec;
        (*encoder).width = width as i32;
        (*encoder).height = height as i32;

        (*stream).time_base = AVRational {
            num: 1,
            den: fps as i32,
        };
        (*encoder).time_base = (*stream).time_base;
        (*encoder).framerate = AVRational {
            num: fps as i32,
            den: 1,
        };

        (*encoder).pix_fmt = AVPixelFormat::AV_PIX_FMT_YUV420P;

        if (*format).flags & AVFMT_GLOBALHEADER == 1 {
            (*encoder).flags |= AV_CODEC_FLAG_GLOBAL_HEADER as i32;
        }

        let mut opts = null_mut();
        av_dict_set(&mut opts, c"preset".as_ptr(), c"ultrafast".as_ptr(), 0);
        av_dict_set(&mut opts, c"tune".as_ptr(), c"zerolatency".as_ptr(), 0);
        avcodec_open2(encoder, codec, &mut opts);

        let bgra_frame = av_frame_alloc();
        (*bgra_frame).format = source_format as i32;
        (*bgra_frame).width = (*encoder).width;
        (*bgra_frame).height = (*encoder).height;
        av_frame_get_buffer(bgra_frame, 0);

        let mut enc_frame = av_frame_alloc();
        (*enc_frame).format = (*encoder).pix_fmt as i32;
        (*enc_frame).width = (*encoder).width;
        (*enc_frame).height = (*encoder).height;
        av_frame_get_buffer(enc_frame, 0);

        avcodec_parameters_from_context((*stream).codecpar, encoder);

        if !((*format).flags & AVFMT_NOFILE == 1) {
            avio_open(&mut (*output).pb, filename, AVIO_FLAG_WRITE);
        }

        avformat_write_header(output, null_mut());

        let sws_ctx = sws_getContext(
            width as i32,
            height as i32,
            source_format,
            width as i32,
            height as i32,
            (*encoder).pix_fmt,
            SWS_BILINEAR,
            null_mut(),
            null_mut(),
            null(),
        );

        let mut packet = av_packet_alloc();

        for (i, chunk) in buf
            .chunks_exact(width as usize * height as usize * 4)
            .enumerate()
        {
            av_frame_make_writable(bgra_frame);

            (*bgra_frame).data[0] = chunk.as_ptr() as *mut u8;

            sws_scale(
                sws_ctx,
                (*bgra_frame).data.as_ptr() as *const *const _,
                (*bgra_frame).linesize.as_ptr(),
                0,
                height as i32,
                (*enc_frame).data.as_ptr(),
                (*enc_frame).linesize.as_ptr(),
            );

            (*enc_frame).pts = i as i64;

            write_frame(output, encoder, stream, enc_frame, packet);
        }

        write_frame(output, encoder, stream, null_mut(), packet);

        av_write_trailer(output);

        if !((*format).flags & AVFMT_NOFILE == 1) {
            avio_closep(&mut (*output).pb);
        }

        avcodec_free_context(&mut encoder);
        av_frame_free(&mut enc_frame);
        av_packet_free(&mut packet);

        avformat_free_context(output);
    }
}

unsafe fn write_frame(
    output: *mut AVFormatContext,
    encoder: *mut AVCodecContext,
    stream: *mut AVStream,
    frame: *mut AVFrame,
    packet: *mut AVPacket,
) {
    avcodec_send_frame(encoder, frame);

    receive_and_write_packets(output, encoder, stream, packet)
}

unsafe fn receive_and_write_packets(
    output: *mut AVFormatContext,
    encoder: *mut AVCodecContext,
    stream: *mut AVStream,
    packet: *mut AVPacket,
) {
    while avcodec_receive_packet(encoder, packet) >= 0 {
        av_packet_rescale_ts(packet, (*encoder).time_base, (*stream).time_base);
        (*packet).stream_index = (*stream).index;

        av_interleaved_write_frame(output, packet);
        av_packet_unref(packet);
    }
}
