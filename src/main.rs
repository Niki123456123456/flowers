use dioxus::html::geometry::WheelDelta;
use dioxus::html::input_data::MouseButton;
use dioxus::html::{FileData, HasFileData};
use dioxus::prelude::*;
use std::path::Path;

const FAVICON: Asset = asset!("/assets/favicon.ico");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let image_src = use_signal(|| None::<String>);
    let image_name = use_signal(|| None::<String>);
    let error = use_signal(|| None::<String>);
    let mut zoom = use_signal(|| 1.0_f64);
    let mut pan = use_signal(|| (0.0_f64, 0.0_f64));
    let mut is_panning = use_signal(|| false);
    let mut last_mouse = use_signal(|| None::<(f64, f64)>);
    let mut is_drag_over = use_signal(|| false);

    rsx! {
        document::Link { rel: "icon", href: FAVICON }

        div {
            ondragover: move |event| {
                event.prevent_default();
                is_drag_over.set(true);
            },
            ondragleave: move |_| {
                is_drag_over.set(false);
            },
            ondrop: move |event| {
                event.prevent_default();
                is_drag_over.set(false);

                let files = event.files();
                let mut image_src = image_src;
                let mut image_name = image_name;
                let mut error = error;
                let mut zoom = zoom;
                let mut pan = pan;
                let mut is_panning = is_panning;
                let mut last_mouse = last_mouse;

                spawn(async move {
                    let Some(file) = files.into_iter().next() else {
                        error.set(Some("Keine Datei erkannt.".to_string()));
                        return;
                    };

                    if !is_supported_image(&file) {
                        error.set(Some("Bitte eine Bilddatei droppen (PNG, JPG, WEBP, GIF, ...).".to_string()));
                        return;
                    }

                    match file.read_bytes().await {
                        Ok(bytes) => {
                            let mime = image_mime_type(&file);
                            let data_uri = format!(
                                "data:{mime};base64,{}",
                                encode_base64(bytes.as_ref())
                            );

                            image_src.set(Some(data_uri));
                            image_name.set(Some(file.name()));
                            zoom.set(1.0);
                            pan.set((0.0, 0.0));
                            is_panning.set(false);
                            last_mouse.set(None);
                            error.set(None);
                        }
                        Err(err) => {
                            error.set(Some(format!("Datei konnte nicht gelesen werden: {err}")));
                        }
                    }
                });
            },
            onwheel: move |event| {
                if !event.modifiers().contains(Modifiers::CONTROL) {
                    return;
                }

                event.prevent_default();

                let delta_y = match event.delta() {
                    WheelDelta::Pixels(delta) => delta.y,
                    WheelDelta::Lines(delta) => delta.y * 30.0,
                    WheelDelta::Pages(delta) => delta.y * 300.0,
                };

                let factor = (-delta_y / 280.0).exp();
                let next_zoom = (zoom() * factor).clamp(0.1, 20.0);
                zoom.set(next_zoom);
            },
            onmousedown: move |event| {
                if image_src().is_none() || event.trigger_button() != Some(MouseButton::Primary) {
                    return;
                }

                event.prevent_default();
                let point = event.client_coordinates();
                is_panning.set(true);
                last_mouse.set(Some((point.x, point.y)));
            },
            onmousemove: move |event| {
                if !is_panning() {
                    return;
                }

                if !event.held_buttons().contains(MouseButton::Primary) {
                    is_panning.set(false);
                    last_mouse.set(None);
                    return;
                }

                let point = event.client_coordinates();
                if let Some((last_x, last_y)) = last_mouse() {
                    let dx = point.x - last_x;
                    let dy = point.y - last_y;
                    pan.with_mut(|(x, y)| {
                        *x += dx;
                        *y += dy;
                    });
                }
                last_mouse.set(Some((point.x, point.y)));
            },
            onmouseup: move |_| {
                is_panning.set(false);
                last_mouse.set(None);
            },
            onmouseleave: move |_| {
                is_panning.set(false);
                last_mouse.set(None);
            },
            style: format!(
                "
                width: 100vw;
                height: 100vh;
                margin: 0;
                overflow: hidden;
                display: flex;
                align-items: center;
                justify-content: center;
                background: linear-gradient(135deg, #101726 0%, #1b2e4b 55%, #254067 100%);
                color: #f2f6ff;
                font-family: system-ui, -apple-system, Segoe UI, sans-serif;
                border: {};
                box-sizing: border-box;
                user-select: none;
                ",
                if is_drag_over() {
                    "6px dashed rgba(255,255,255,0.75)"
                } else {
                    "6px dashed transparent"
                }
            ),

            if let Some(src) = image_src() {
                div {
                    style: "
                        width: 100%;
                        height: 100%;
                        display: flex;
                        flex-direction: column;
                        align-items: center;
                        justify-content: center;
                        gap: 16px;
                        padding: 20px;
                        box-sizing: border-box;
                    ",
                    div {
                        style: "
                            width: 100%;
                            flex: 1;
                            display: flex;
                            align-items: center;
                            justify-content: center;
                            overflow: hidden;
                        ",
                        div {
                            style: format!(
                                "
                                transform: translate({}px, {}px);
                                will-change: transform;
                                ",
                                pan().0,
                                pan().1
                            ),
                            img {
                                src: src,
                                alt: image_name().unwrap_or_else(|| "Bild".to_string()),
                                draggable: "false",
                                style: format!(
                                    "
                                    max-width: 98vw;
                                    max-height: 84vh;
                                    object-fit: contain;
                                    transform: scale({});
                                    transform-origin: center center;
                                    transition: transform 0.06s linear;
                                    cursor: {};
                                    ",
                                    zoom(),
                                    if is_panning() { "grabbing" } else { "grab" }
                                ),
                            }
                        }
                    }
                    p {
                        style: "margin: 0; font-size: 14px; opacity: 0.9;",
                        "{image_name().unwrap_or_else(|| \"Bild\".to_string())} | Zoom: {(zoom() * 100.0).round()}% (Strg + Mausrad) | Verschieben: Linke Maustaste halten + ziehen"
                    }
                }
            } else {
                div {
                    style: "
                        text-align: center;
                        padding: 24px;
                        max-width: 680px;
                        border-radius: 20px;
                        background: rgba(15, 20, 32, 0.35);
                        border: 1px solid rgba(255, 255, 255, 0.2);
                        backdrop-filter: blur(6px);
                    ",
                    h1 {
                        style: "margin: 0 0 12px 0; font-size: clamp(1.4rem, 2.8vw, 2.2rem);",
                        "Bild-Dropzone"
                    }
                    p {
                        style: "margin: 0; line-height: 1.5; opacity: 0.95;",
                        "Ziehe ein Bild aus dem Explorer in dieses Fenster. Es wird direkt im Browser geladen und groß dargestellt."
                    }
                    p {
                        style: "margin: 12px 0 0 0; font-size: 14px; opacity: 0.85;",
                        "Zoom im Bild: Strg + Mausrad"
                    }
                }
            }

            if let Some(message) = error() {
                div {
                    style: "
                        position: fixed;
                        left: 16px;
                        bottom: 16px;
                        max-width: min(80vw, 560px);
                        padding: 10px 14px;
                        border-radius: 10px;
                        background: rgba(180, 38, 45, 0.95);
                        color: #fff;
                        font-size: 14px;
                    ",
                    "{message}"
                }
            }
        }
    }
}

fn image_mime_type(file: &FileData) -> String {
    if let Some(content_type) = file.content_type() {
        if content_type.starts_with("image/") {
            return content_type;
        }
    }

    let extension = Path::new(&file.name())
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());

    match extension.as_deref() {
        Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
        Some("png") => "image/png".to_string(),
        Some("gif") => "image/gif".to_string(),
        Some("webp") => "image/webp".to_string(),
        Some("bmp") => "image/bmp".to_string(),
        Some("svg") => "image/svg+xml".to_string(),
        Some("avif") => "image/avif".to_string(),
        _ => "image/*".to_string(),
    }
}

fn is_supported_image(file: &FileData) -> bool {
    if let Some(content_type) = file.content_type() {
        if content_type.starts_with("image/") {
            return true;
        }
    }

    let extension = Path::new(&file.name())
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase());

    matches!(
        extension.as_deref(),
        Some("jpg")
            | Some("jpeg")
            | Some("png")
            | Some("gif")
            | Some("webp")
            | Some("bmp")
            | Some("svg")
            | Some("avif")
    )
}

fn encode_base64(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);

        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);

        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }

        if chunk.len() > 2 {
            out.push(TABLE[(n & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }

    out
}
