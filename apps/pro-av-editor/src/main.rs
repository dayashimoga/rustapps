#![deny(unsafe_code)]
//! Professional Audio & Video Editor Desktop / CLI Entrypoint

use pave_core::{
    AnimatedProperty, BlendMode, Clip, ClipType, ColorGradingSettings, CompressorSettings,
    EditorService, ParametricEqSettings, Transform2D,
};

fn main() {
    println!("========================================================");
    println!("  Professional Audio & Video Editor - 100% Offline NLE ");
    println!("========================================================");

    let service = match EditorService::from_template("tpl_standard_video", "Flagship Studio Demo") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize EditorService: {e}");
            return;
        }
    };

    let proj = service.get_project().expect("Failed to get project state");
    println!(
        "Project Initialized: \"{}\" (Tracks: {}, Canvas: {}x{} @ {:.2} FPS)",
        proj.name,
        proj.tracks.len(),
        proj.settings.width,
        proj.settings.height,
        proj.settings.frame_rate.as_f64()
    );

    // Add sample video clip
    let clip1 = Clip {
        id: "clip-demo-1".to_string(),
        name: "Main Camera A-Roll".to_string(),
        clip_type: ClipType::Video,
        track_id: "v1".to_string(),
        source_id: Some("media_raw_001".to_string()),
        source_in_ms: 0,
        source_out_ms: 6000,
        timeline_in_ms: 0,
        timeline_out_ms: 6000,
        speed_multiplier: 1.0,
        volume: AnimatedProperty::constant(1.0),
        pan: 0.0,
        opacity: AnimatedProperty::constant(1.0),
        blend_mode: BlendMode::Normal,
        transform: Transform2D::default(),
        color_grading: ColorGradingSettings::default(),
        eq_settings: ParametricEqSettings::default(),
        compressor: CompressorSettings::default(),
    };

    if let Err(e) = service.add_clip(clip1) {
        eprintln!("Error adding clip: {e}");
    } else {
        println!("Successfully added clip on track V1 (Duration: 6.00s)");
    }

    // Render preview frame at 2.5s
    match service.render_frame(2500, 320, 180) {
        Ok(frame) => {
            println!(
                "Rendered preview frame at 2.50s ({}x{}, {} bytes)",
                frame.width,
                frame.height,
                frame.data.len()
            );
        }
        Err(e) => eprintln!("Frame render error: {e}"),
    }

    // Compute scopes
    match service.get_video_scopes(2500, 320, 180) {
        Ok(scopes) => {
            println!(
                "Video Scopes computed: Waveform {} cols x {} rows, Vectorscope {}x{}",
                scopes.waveform_cols,
                scopes.waveform_rows,
                scopes.vectorscope_dim,
                scopes.vectorscope_dim
            );
        }
        Err(e) => eprintln!("Scopes error: {e}"),
    }

    // Export EDL
    match service.export_edl() {
        Ok(edl) => {
            println!(
                "\nGenerated CMX 3600 Edit Decision List (EDL):\n{}",
                edl.trim()
            );
        }
        Err(e) => eprintln!("EDL export error: {e}"),
    }

    println!("\nPAVE studio engine operational and ready for desktop/mobile.");
}
