use std::{
    io::Write,
    ptr::{null, null_mut},
};

use ffmpeg_next::ffi::*;
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
        let mut oc = null_mut();

        let filename = c"output.mp4".as_ptr();

        avformat_alloc_output_context2(&mut oc, null(), null(), filename);

        let fmt = (*oc).oformat;

        let codec = avcodec_find_encoder_by_name(c"libx264".as_ptr());

        let mut tmp_pkt = av_packet_alloc();

        let st = avformat_new_stream(oc, null());
        (*st).id = ((*oc).nb_streams - 1) as i32;

        let mut enc = avcodec_alloc_context3(codec);

        (*enc).codec_id = (*fmt).video_codec;
        (*enc).width = width as i32;
        (*enc).height = height as i32;

        (*st).time_base = AVRational {
            num: 1,
            den: fps as i32,
        };
        (*enc).time_base = (*st).time_base;

        (*enc).pix_fmt = AVPixelFormat::AV_PIX_FMT_YUV420P;

        if (*(*oc).oformat).flags & AVFMT_GLOBALHEADER == 1 {
            (*enc).flags |= AV_CODEC_FLAG_GLOBAL_HEADER as i32;
        }

        avcodec_open2(enc, codec, null_mut());

        let bgra_frame = av_frame_alloc();
        (*bgra_frame).format = source_format as i32;
        (*bgra_frame).width = (*enc).width;
        (*bgra_frame).height = (*enc).height;
        av_frame_get_buffer(bgra_frame, 0);

        let mut enc_frame = av_frame_alloc();
        (*enc_frame).format = (*enc).pix_fmt as i32;
        (*enc_frame).width = (*enc).width;
        (*enc_frame).height = (*enc).height;

        av_frame_get_buffer(enc_frame, 0);

        avcodec_parameters_from_context((*st).codecpar, enc);

        if !((*fmt).flags & AVFMT_NOFILE == 1) {
            avio_open(&mut (*oc).pb, filename, AVIO_FLAG_WRITE);
        }

        avformat_write_header(oc, null_mut());

        let sws_ctx = sws_getContext(
            width as i32,
            height as i32,
            source_format,
            width as i32,
            height as i32,
            (*enc).pix_fmt,
            SWS_BILINEAR,
            null_mut(),
            null_mut(),
            null(),
        );

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

            write_frame(oc, enc, st, enc_frame, tmp_pkt);
        }

        write_frame(oc, enc, st, null_mut(), tmp_pkt);

        av_write_trailer(oc);

        if !((*fmt).flags & AVFMT_NOFILE == 1) {
            avio_closep(&mut (*oc).pb);
        }

        avcodec_free_context(&mut enc);
        av_frame_free(&mut enc_frame);
        av_packet_free(&mut tmp_pkt);

        avformat_free_context(oc);
    }
}

unsafe fn write_frame(
    fmt_ctx: *mut AVFormatContext,
    enc_ctx: *mut AVCodecContext,
    stream: *mut AVStream,
    frame: *mut AVFrame,
    pkt: *mut AVPacket,
) {
    avcodec_send_frame(enc_ctx, frame);

    receive_and_write_packets(fmt_ctx, enc_ctx, stream, pkt)
}

unsafe fn receive_and_write_packets(
    fmt_ctx: *mut AVFormatContext,
    enc_ctx: *mut AVCodecContext,
    stream: *mut AVStream,
    pkt: *mut AVPacket,
) {
    while avcodec_receive_packet(enc_ctx, pkt) >= 0 {
        av_packet_rescale_ts(pkt, (*enc_ctx).time_base, (*stream).time_base);
        (*pkt).stream_index = (*stream).index;

        av_interleaved_write_frame(fmt_ctx, pkt);
        av_packet_unref(pkt);
    }
}
