use super::*;
use crate::{
    editor::{Content, PathRendering, Pattern, Point, TextLayout, Tool},
    frame::{Frame, Rect},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::io::{Cursor, Read};
use zip::ZipArchive;

fn fixture() -> Value {
    let frame = Frame::new(80, 60, [11, 22, 33, 255].repeat(4800)).unwrap();
    let mut png = vec![];
    frame.write_png(&mut png).unwrap();
    json!({"format":"neo-snap","version":1,"width":80,"height":60,
        "background":"base","cropRect":{"x":0,"y":0,"width":80,"height":60},
        "assets":[{"id":"base","dataUrl":format!("data:image/png;base64,{}", STANDARD.encode(png))}],
        "objects":[]})
}
fn object(tool: &str) -> Value {
    json!({"tool":tool,"x1":5,"y1":10,"x2":65,"y2":40,
        "color":"#ef3340","size":5,"opacity":0.4,"stroke":"dashed"})
}
fn import(v: &Value) -> Result<Project, String> {
    decode(&serde_json::to_vec(v).unwrap(), 32 * 1024 * 1024).map_err(|e| e.message(false))
}

#[test]
fn imported_source_pixels_crop_and_unsaved_state_survive_native_round_trip() {
    let mut v = fixture();
    v["cropRect"] = json!({"x":3,"y":4,"width":60,"height":40});
    let p = import(&v).unwrap();
    assert_eq!(p.frame.width, 80);
    assert_eq!(&p.frame.bgra[..4], &[11, 22, 33, 255]);
    assert_eq!(p.document.crop, Rect::new(3, 4, 60, 40).unwrap());
    assert!(!p.document.is_project_saved());
    let bytes = crate::project::encode(&p.frame, &p.document).unwrap();
    let reopened = crate::project::decode(&bytes).unwrap();
    assert_eq!(reopened.frame, p.frame);
    assert_eq!(reopened.document.crop, p.document.crop);
}

#[test]
fn native_shapes_keep_color_curve_pattern_opacity_and_empty_history() {
    let mut v = fixture();
    let mut line = object("line");
    line["curve"] = json!({"x":35,"y":2});
    let mut fill = object("ellipseFill");
    fill.as_object_mut().unwrap().remove("stroke");
    fill["opacity"] = json!(1);
    v["objects"] = json!([line, fill]);
    let mut p = import(&v).unwrap();
    let a = &p.document.objects[0];
    assert_eq!(a.tool, Tool::Line);
    assert_eq!(
        a.points.as_ref(),
        &vec![
            Point::new(5., 10.),
            Point::new(65., 40.),
            Point::new(35., 2.)
        ]
    );
    assert_eq!(a.style.rgb, [239, 51, 64]);
    assert_eq!(a.style.width, 5);
    assert_eq!(a.pattern, Pattern::Dashed);
    assert_eq!(a.opacity, 40);
    assert_eq!(p.document.objects[1].tool, Tool::FilledEllipse);
    assert!(!p.document.undo());
    assert!(!p.document.redo());
    assert_eq!(p.document.selected, None);
}

#[test]
fn unsupported_hidden_object_rejects_the_entire_document_without_leaking_text() {
    let mut v = fixture();
    let mut text = object("future-private-tool");
    text["text"] = json!("secret account data");
    text["opacity"] = json!(0);
    text["x1"] = json!(1000);
    v["objects"] = json!([object("box"), text]);
    let err = import(&v).err().expect("must reject entire file");
    assert!(err.contains("2"), "{err}");
    assert!(!err.contains("secret"), "{err}");
}

#[test]
fn dotnet_text_imports_editable_content_layout_and_effects() {
    let mut v = fixture();
    let mut text = object("text");
    text.as_object_mut().unwrap().remove("x2");
    text.as_object_mut().unwrap().remove("y2");
    text.as_object_mut().unwrap().remove("stroke");
    text["x1"] = json!(12);
    text["y1"] = json!(14);
    text["size"] = json!(6.5);
    text["opacity"] = json!(0.75);
    text["text"] = json!("ภาษาไทย\nEnglish");
    text["outline"] = json!({"enabled":true,"color":"#102030","width":3});
    text["shadow"] = json!({"enabled":true,"color":"#405060","blur":6,"x":-3,"y":4});
    text["bubbleFill"] = json!("#ffffff");
    text["bubbleBorder"] = json!("#abcdef");
    text["bubbleBorderWidth"] = json!(5);
    v["objects"] = json!([text]);

    let p = import(&v).unwrap();
    let text = &p.document.objects[0];
    assert_eq!(text.tool, Tool::Text);
    assert_eq!(text.rendering, PathRendering::DotNet);
    assert_eq!(text.points.as_ref(), &vec![Point::new(12., 14.)]);
    assert_eq!(text.opacity, 75);
    assert_eq!(text.effects.outline.rgb, [16, 32, 48]);
    assert_eq!(text.effects.outline.width, 3);
    assert!(text.effects.outline.enabled);
    assert!(text.effects.shadow.enabled);
    let Content::Text {
        text: value,
        size,
        wrap_width,
        background,
        layout,
        mask,
        ..
    } = &text.content
    else {
        panic!("expected imported text")
    };
    assert_eq!(value.as_ref(), "ภาษาไทย\nEnglish");
    assert_eq!(*size, 26);
    assert_eq!(*wrap_width, None);
    assert!(!background.enabled);
    assert_eq!(
        *layout,
        TextLayout::DotNet {
            source_size: 6.5,
            bubble: None,
        }
    );
    assert!(mask.width > 0 && mask.height > 0);
}

#[test]
fn dotnet_bubble_imports_independent_frame_border_and_tail() {
    let mut v = fixture();
    let mut bubble = object("bubble");
    bubble.as_object_mut().unwrap().remove("x2");
    bubble.as_object_mut().unwrap().remove("y2");
    bubble.as_object_mut().unwrap().remove("stroke");
    bubble["x1"] = json!(8);
    bubble["y1"] = json!(6);
    bubble["size"] = json!(5);
    bubble["opacity"] = json!(1);
    bubble["text"] = json!("ข้อความทดสอบ");
    bubble["bubbleWidth"] = json!(60);
    bubble["bubbleHeight"] = json!(35);
    bubble["bubbleFill"] = json!("#f0f1f2");
    bubble["bubbleBorder"] = json!("#112233");
    bubble["bubbleBorderWidth"] = json!(4);
    bubble["tail"] = json!({"x":20,"y":50});
    v["objects"] = json!([bubble]);

    let p = import(&v).unwrap();
    let bubble = &p.document.objects[0];
    assert_eq!(bubble.tool, Tool::Callout);
    assert_eq!(bubble.rendering, PathRendering::DotNet);
    assert_eq!(bubble.points.as_ref(), &vec![Point::new(8., 6.)]);
    assert_eq!(bubble.tail_tip(), Some(Point::new(-8., 56.)));
    let Content::Text {
        text,
        size,
        wrap_width,
        background,
        layout,
        tail,
        ..
    } = &bubble.content
    else {
        panic!("expected imported bubble")
    };
    assert_eq!(text.as_ref(), "ข้อความทดสอบ");
    assert_eq!(*size, 20);
    assert_eq!(*wrap_width, Some(36));
    assert_eq!(background.rgb, [240, 241, 242]);
    assert!(background.enabled);
    assert_eq!(*tail, Some(Point::new(20., 50.)));
    let TextLayout::DotNet {
        source_size,
        bubble: Some(frame),
    } = *layout
    else {
        panic!("expected retained .NET bubble frame")
    };
    assert_eq!(source_size, 5.);
    assert_eq!(frame.width, 60.);
    assert_eq!(frame.min_height, 35.);
    assert_eq!(frame.padding, 12.);
    assert_eq!(frame.border_rgb, [17, 34, 51]);
    assert_eq!(frame.border_width, 4.);
}

#[test]
fn dotnet_bubble_moves_an_inside_tail_to_the_nearest_exterior_edge() {
    let mut v = fixture();
    let mut bubble = object("bubble");
    bubble.as_object_mut().unwrap().remove("x2");
    bubble.as_object_mut().unwrap().remove("y2");
    bubble.as_object_mut().unwrap().remove("stroke");
    bubble["x1"] = json!(8);
    bubble["y1"] = json!(20);
    bubble["size"] = json!(5);
    bubble["text"] = json!("A");
    bubble["bubbleWidth"] = json!(60);
    bubble["bubbleHeight"] = json!(50);
    bubble["tail"] = json!({"x":30,"y":5});
    v["objects"] = json!([bubble]);

    let p = import(&v).unwrap();
    assert_eq!(p.document.objects[0].tail_tip(), Some(Point::new(38., 4.)));
}

#[test]
fn dotnet_bubble_renders_its_independent_border_and_fill() {
    let mut v = fixture();
    let mut bubble = object("bubble");
    bubble.as_object_mut().unwrap().remove("x2");
    bubble.as_object_mut().unwrap().remove("y2");
    bubble.as_object_mut().unwrap().remove("stroke");
    bubble["x1"] = json!(8);
    bubble["y1"] = json!(6);
    bubble["size"] = json!(5);
    bubble["opacity"] = json!(1);
    bubble["text"] = json!("A");
    bubble["bubbleWidth"] = json!(60);
    bubble["bubbleHeight"] = json!(35);
    bubble["bubbleFill"] = json!("#f0f1f2");
    bubble["bubbleBorder"] = json!("#112233");
    bubble["bubbleBorderWidth"] = json!(4);
    bubble["tail"] = json!({"x":30,"y":50});
    v["objects"] = json!([bubble]);

    let p = import(&v).unwrap();
    let output = crate::render::flatten(&p.frame, &p.document).unwrap();
    let pixel = |x: usize, y: usize| {
        let i = (y * output.width as usize + x) * 4;
        &output.bgra[i..i + 4]
    };
    assert_eq!(pixel(8, 22), &[51, 34, 17, 255]);
    assert_eq!(pixel(50, 22), &[242, 241, 240, 255]);
}

#[test]
fn imported_text_and_bubble_round_trip_in_schema_five_with_default_font() {
    let mut v = fixture();
    let mut text = object("text");
    text.as_object_mut().unwrap().remove("x2");
    text.as_object_mut().unwrap().remove("y2");
    text.as_object_mut().unwrap().remove("stroke");
    text["text"] = json!("ไทย English");
    text["size"] = json!(5);
    let mut bubble = text.clone();
    bubble["tool"] = json!("bubble");
    bubble["x1"] = json!(20);
    bubble["y1"] = json!(20);
    bubble["text"] = json!("Bubble");
    bubble["bubbleWidth"] = json!(55);
    bubble["bubbleHeight"] = json!(30);
    bubble["bubbleFill"] = json!("#fefefe");
    bubble["bubbleBorder"] = json!("#123456");
    bubble["bubbleBorderWidth"] = json!(3);
    v["objects"] = json!([text, bubble]);

    let p = import(&v).unwrap();
    let bytes = crate::project::encode(&p.frame, &p.document).unwrap();
    let mut archive = ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut manifest = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    let manifest: Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(manifest["schema"], 5);
    assert_eq!(manifest["objects"][0]["content"]["font"], "noto_sans_thai");
    drop(archive);

    let reopened = crate::project::decode(&bytes).unwrap();
    assert_eq!(reopened.document.objects, p.document.objects);
    assert_eq!(
        crate::render::flatten(&reopened.frame, &reopened.document).unwrap(),
        crate::render::flatten(&p.frame, &p.document).unwrap()
    );
}

#[test]
fn imported_text_and_bubble_remain_editable_through_full_size_and_frame_range() {
    let mut v = fixture();
    let mut text = object("text");
    text.as_object_mut().unwrap().remove("x2");
    text.as_object_mut().unwrap().remove("y2");
    text.as_object_mut().unwrap().remove("stroke");
    text["text"] = json!("Maximum");
    text["size"] = json!(64);
    let mut bubble = object("bubble");
    bubble.as_object_mut().unwrap().remove("x2");
    bubble.as_object_mut().unwrap().remove("y2");
    bubble.as_object_mut().unwrap().remove("stroke");
    bubble["x1"] = json!(8);
    bubble["y1"] = json!(6);
    bubble["size"] = json!(5);
    bubble["text"] = json!("Bubble text");
    bubble["bubbleWidth"] = json!(60);
    bubble["bubbleHeight"] = json!(35);
    bubble["tail"] = json!({"x":20,"y":50});
    v["objects"] = json!([text, bubble]);

    let mut p = import(&v).unwrap();
    p.document.selected = Some(p.document.objects[0].id);
    assert!(p.document.edit_text("Changed", 256).unwrap());
    assert!(matches!(
        p.document.objects[0].content,
        Content::Text {
            size: 256,
            layout: TextLayout::DotNet {
                source_size: 64.,
                bubble: None
            },
            ..
        }
    ));
    assert!(p.document.undo());

    p.document.selected = Some(p.document.objects[1].id);
    assert!(
        p.document
            .resize_text(20, Some(80), Point::new(8., 6.), false, false)
            .unwrap()
    );
    let Content::Text {
        size,
        wrap_width,
        layout: TextLayout::DotNet {
            bubble: Some(frame),
            ..
        },
        tail,
        ..
    } = p.document.objects[1].content
    else {
        panic!("expected resized imported bubble")
    };
    assert_eq!(size, 20);
    assert_eq!(frame.width, 80.);
    assert_eq!(frame.min_height, 35.);
    assert_eq!(frame.padding, 14.);
    assert_eq!(wrap_width, Some(52));
    assert_eq!(tail, Some(Point::new(20., 50.)));
    assert!(p.document.undo());

    assert!(
        p.document
            .resize_text(40, Some(120), Point::new(8., 6.), false, false)
            .unwrap()
    );
    let Content::Text {
        size,
        layout:
            TextLayout::DotNet {
                source_size,
                bubble: Some(frame),
            },
        tail,
        ..
    } = p.document.objects[1].content
    else {
        panic!("expected scaled imported bubble")
    };
    assert_eq!(size, 40);
    assert_eq!(source_size, 10.);
    assert_eq!(frame.width, 120.);
    assert_eq!(frame.min_height, 70.);
    assert_eq!(tail, Some(Point::new(40., 100.)));
    assert!(p.document.undo());
    let text_color = p.document.objects[1].style.rgb;
    assert!(p.document.set_callout_border(crate::editor::Style {
        rgb: [90, 80, 70],
        width: 7,
    }));
    assert_eq!(p.document.objects[1].style.rgb, text_color);
    assert_eq!(
        p.document.objects[1].callout_border(),
        crate::editor::Style {
            rgb: [90, 80, 70],
            width: 7,
        }
    );
    assert!(p.document.undo());
    assert_eq!(
        p.document.objects[1].callout_border(),
        crate::editor::Style {
            rgb: [239, 51, 64],
            width: 5,
        }
    );
    assert_eq!(
        p.document.objects[1],
        import(&v).unwrap().document.objects[1]
    );
}

#[test]
fn strict_input_rejects_version_unknown_fields_fractional_values_and_bad_assets() {
    for (field, value) in [
        ("version", json!(2)),
        ("format", json!("other")),
        ("width", json!(80.5)),
    ] {
        let mut v = fixture();
        v[field] = value;
        assert!(import(&v).is_err(), "{field}");
    }
    let mut v = fixture();
    v["objects"] = json!([object("box")]);
    for (field, value) in [
        ("size", json!(41)),
        ("size", json!(5.5)),
        ("opacity", json!(0.12345)),
        ("rotation", json!(45)),
    ] {
        let mut bad = v.clone();
        bad["objects"][0][field] = value;
        assert!(import(&bad).is_err(), "{field}");
    }
    v["assets"][0]["dataUrl"] = json!("https://example.invalid/image.png");
    assert!(import(&v).is_err());
    let v = fixture();
    assert!(decode(&serde_json::to_vec(&v).unwrap(), 80 * 60 * 4 - 1).is_err());
}

#[test]
fn invalid_text_or_bubble_rejects_the_whole_file_without_echoing_content() {
    let cases = [
        ("text", json!("   "), "size", json!(5)),
        ("text", json!("private"), "size", json!(65)),
        ("bubble", json!("private"), "bubbleWidth", json!(19)),
        ("bubble", json!("private"), "bubbleBorderWidth", json!(17)),
        ("bubble", json!("private"), "tail", json!({"x":32768,"y":0})),
    ];
    for (tool, content, field, value) in cases {
        let mut v = fixture();
        let mut invalid = object(tool);
        invalid.as_object_mut().unwrap().remove("x2");
        invalid.as_object_mut().unwrap().remove("y2");
        invalid.as_object_mut().unwrap().remove("stroke");
        invalid["text"] = content;
        invalid[field] = value;
        v["objects"] = json!([object("box"), invalid]);
        let error = import(&v).err().expect("whole import must fail");
        assert!(!error.contains("private"), "{field}: {error}");
    }

    let mut v = fixture();
    let mut long = object("text");
    long.as_object_mut().unwrap().remove("x2");
    long.as_object_mut().unwrap().remove("y2");
    long.as_object_mut().unwrap().remove("stroke");
    long["text"] = json!("x".repeat(4097));
    v["objects"] = json!([long]);
    assert!(import(&v).is_err());
}

#[test]
fn cropped_image_keeps_original_pixels_and_editable_native_viewport() {
    let mut v = fixture();
    let mut image = object("image");
    image["id"] = json!("photo");
    image["assetId"] = json!("base");
    image["sourceCrop"] = json!({"x":10.25,"y":9.5,"width":60,"height":30});
    image["opacity"] = json!(1);
    image.as_object_mut().unwrap().remove("stroke");
    v["objects"] = json!([image]);
    let p = import(&v).unwrap();
    match &p.document.objects[0].content {
        Content::Group { image } => {
            assert_eq!(image.frame.width, 80);
            assert_eq!(image.viewport.left, 10.25);
            assert_eq!(image.viewport.top, 9.5);
            assert_eq!(image.viewport.right, 70.25);
        }
        _ => panic!("expected retained source group"),
    }
    let bytes = crate::project::encode(&p.frame, &p.document).unwrap();
    assert!(crate::project::decode(&bytes).is_ok());
}

#[test]
fn duplicate_keys_and_excessive_nesting_are_rejected_before_conversion() {
    let bytes = serde_json::to_string(&fixture()).unwrap();
    let duplicate = bytes.replacen("\"version\":1", "\"version\":1,\"version\":1", 1);
    assert!(decode(duplicate.as_bytes(), 32 * 1024 * 1024).is_err());
    let nested = format!("{}0{}", "[".repeat(17), "]".repeat(17));
    assert!(decode(nested.as_bytes(), 32 * 1024 * 1024).is_err());
}

#[test]
fn required_background_and_invalid_owned_clip_are_not_silently_defaulted() {
    let mut v = fixture();
    v.as_object_mut().unwrap().remove("background");
    assert!(
        import(&v).is_err(),
        "missing required background must not become a white canvas"
    );
    let mut v = fixture();
    let mut image = object("image");
    image["id"] = json!("photo");
    image["assetId"] = json!("base");
    image["sourceCrop"] = json!({"x":0,"y":0,"width":60,"height":30});
    image["opacity"] = json!(1);
    image.as_object_mut().unwrap().remove("stroke");
    let mut child = object("box");
    child["owner"] = json!("photo");
    child["clipRect"] = json!({"x":0,"y":0,"width":-1,"height":20});
    v["objects"] = json!([image, child]);
    assert!(import(&v).is_err(), "invalid stored clip must be rejected");
}

#[test]
fn compatible_owned_annotations_preserve_visible_geometry_and_order() {
    let mut v = fixture();
    let mut image = object("image");
    image["id"] = json!("photo");
    image["assetId"] = json!("base");
    image["sourceCrop"] = json!({"x":10,"y":10,"width":60,"height":30});
    image["opacity"] = json!(1);
    image.as_object_mut().unwrap().remove("stroke");
    let mut child = object("box");
    child["owner"] = json!("photo");
    child["childId"] = json!("annotation");
    child["x1"] = json!(15);
    child["y1"] = json!(20);
    child["x2"] = json!(35);
    child["y2"] = json!(30);
    v["objects"] = json!([image.clone(), child.clone(), object("line")]);
    let p = import(&v).unwrap();
    let objects = &p.document.objects;
    assert_eq!(objects[1].parent, Some(objects[0].id));
    let a = p
        .document
        .parent_transform(objects[1].id)
        .point(objects[1].points[0]);
    assert_eq!(a, Point::new(15., 20.));
    assert_eq!(objects[2].parent, None);
    let bytes = crate::project::encode(&p.frame, &p.document).unwrap();
    let reopened = crate::project::decode(&bytes).unwrap();
    assert_eq!(reopened.document.objects, objects.clone());
    v["objects"] = json!([image, object("line"), child]);
    assert!(
        import(&v).is_err(),
        "interleaved roots cannot silently move under a group"
    );
}

#[test]
fn owned_annotation_accepts_fractional_stored_clip_because_owner_bounds_take_precedence() {
    let mut v = fixture();
    let mut image = object("image");
    image["id"] = json!("photo");
    image["assetId"] = json!("base");
    image["sourceCrop"] = json!({"x":0,"y":0,"width":80,"height":60});
    image["x1"] = json!(0);
    image["y1"] = json!(0);
    image["x2"] = json!(80);
    image["y2"] = json!(60);
    image["opacity"] = json!(1);
    image.as_object_mut().unwrap().remove("stroke");

    let mut child = object("box");
    child["owner"] = json!("photo");
    child["childId"] = json!("annotation");
    child["clipRect"] = json!({"x":0.25,"y":0.5,"width":79.5,"height":59.25});
    v["objects"] = json!([image, child]);

    let p = import(&v).unwrap();
    assert_eq!(p.document.objects[1].parent, Some(p.document.objects[0].id));
}

#[test]
fn diagnostics_are_bounded_localized_and_never_echo_untrusted_values() {
    let mut v = fixture();
    let mut bad = object("unknown secret tool");
    bad["text"] = json!("private patient details");
    v["objects"] = Value::Array(vec![bad; 15]);
    let e = decode(&serde_json::to_vec(&v).unwrap(), 32 * 1024 * 1024)
        .err()
        .unwrap();
    for thai in [false, true] {
        let message = e.message(thai);
        assert!(!message.contains("secret"));
        assert!(!message.contains("patient"));
        assert!(message.contains("10"));
        assert!(message.contains("5"));
        assert!(!message.contains("Object 11:"));
    }
    assert_ne!(e.message(false), e.message(true));
}

#[test]
fn supported_shape_types_and_compatible_effects_keep_editability() {
    for (name, want) in [
        ("box", Tool::Rectangle),
        ("block", Tool::FilledRectangle),
        ("ellipse", Tool::Ellipse),
        ("ellipseFill", Tool::FilledEllipse),
        ("line", Tool::Line),
        ("pen", Tool::Pen),
        ("highlight", Tool::Highlighter),
    ] {
        let mut v = fixture();
        let mut o = object(name);
        o.as_object_mut().unwrap().remove("stroke");
        if matches!(want, Tool::Pen | Tool::Highlighter) {
            o["points"] = json!([{"x":5,"y":10},{"x":65,"y":40}]);
        }
        o["shadow"] = json!({"enabled":true,"color":"#123456","blur":6,"x":-3,"y":4});
        v["objects"] = json!([o]);
        let mut p = import(&v).unwrap();
        assert_eq!(p.document.objects[0].tool, want);
        assert_eq!(p.document.objects[0].effects.shadow.rgb, [18, 52, 86]);
        p.document.selected = Some(p.document.objects[0].id);
        let original = p.document.objects[0].clone();
        assert!(p.document.set_opacity(75));
        assert!(p.document.undo());
        assert_eq!(p.document.objects[0], original);
        assert!(!p.document.is_project_saved());
    }
}

#[test]
fn missing_duplicate_and_oversized_assets_fail_before_publishing_a_project() {
    let mut v = fixture();
    let duplicate = v["assets"][0].clone();
    v["assets"].as_array_mut().unwrap().push(duplicate);
    assert!(import(&v).is_err());
    let mut v = fixture();
    v["background"] = json!("missing");
    assert!(import(&v).is_err());
    let mut v = fixture();
    v["cropRect"]["x"] = json!(0.25);
    assert!(import(&v).is_err());
    let mut v = fixture();
    v["cropRect"]["width"] = json!(81);
    assert!(import(&v).is_err());
    let mut v = fixture();
    v["objects"] = Value::Array(vec![object("box"); 2001]);
    assert!(import(&v).is_err());
    let mut v = fixture();
    let mut o = object("box");
    o["shadow"] = json!({"enabled":false,"color":"#000000","blur":33,"x":3,"y":3});
    v["objects"] = json!([o]);
    assert!(import(&v).is_err());
}

#[test]
fn null_background_is_white_and_does_not_consume_a_full_hidden_canvas() {
    let mut v = fixture();
    v["background"] = Value::Null;
    v["assets"] = json!([]);
    let p = import(&v).unwrap();
    assert_eq!(p.frame.bgra, vec![255; 4]);
    assert_eq!(p.document.source_clip, None);
    let output = crate::render::flatten(&p.frame, &p.document).unwrap();
    assert_eq!(output.width, 80);
    assert!(output.bgra.iter().all(|b| *b == 255));
}

#[test]
fn dotnet_smoothed_paths_and_arrowheads_survive_native_round_trip() {
    let mut v = fixture();
    let mut pen = object("pen");
    pen["points"] = json!([
        {"x":5,"y":10},
        {"x":25,"y":45},
        {"x":45,"y":5},
        {"x":65,"y":40}
    ]);
    pen.as_object_mut().unwrap().remove("stroke");
    let mut arrow = object("arrow");
    arrow["stroke"] = json!("dotted");
    v["objects"] = json!([pen, arrow]);

    let p = import(&v).unwrap();
    assert_eq!(p.document.objects[0].rendering, PathRendering::DotNet);
    assert_eq!(p.document.objects[1].rendering, PathRendering::DotNet);
    assert_eq!(p.document.objects[0].points.len(), 4);
    let sampled = crate::geometry::polyline(&p.document.objects[0]);
    assert!(
        sampled.len() > 4,
        "the .NET path must be smoothed, not joined as raw segments"
    );
    assert!(!sampled.contains(&Point::new(25., 45.)));
    let head = crate::geometry::arrowhead(&p.document.objects[1]);
    for (_, tip) in head {
        let end = p.document.objects[1].points[1];
        assert!(((end.x - tip.x).hypot(end.y - tip.y) - 25.).abs() < 0.000001);
    }

    let bytes = crate::project::encode(&p.frame, &p.document).unwrap();
    let reopened = crate::project::decode(&bytes).unwrap();
    assert_eq!(reopened.document.objects, p.document.objects);
    assert_eq!(
        crate::render::flatten(&reopened.frame, &reopened.document).unwrap(),
        crate::render::flatten(&p.frame, &p.document).unwrap()
    );
}

#[test]
fn scaled_image_children_restore_local_integer_styles_and_ignore_known_irrelevant_defaults() {
    let mut v = fixture();
    let mut image = object("image");
    image["id"] = json!("photo");
    image["assetId"] = json!("base");
    image["sourceCrop"] = json!({"x":0,"y":0,"width":80,"height":60});
    image["x1"] = json!(0);
    image["y1"] = json!(0);
    image["x2"] = json!(57.07179679231581);
    image["y2"] = json!(42.80384759423686);
    image["opacity"] = json!(1);
    image.as_object_mut().unwrap().remove("stroke");

    let mut child = object("pen");
    child["owner"] = json!("photo");
    child["childId"] = json!("mark");
    child["points"] = json!([{"x":5,"y":5},{"x":15,"y":20},{"x":30,"y":10}]);
    child["size"] = json!(7.133974599039476);
    child["shadow"] = json!({"enabled":true,"color":"#123456","blur":4.280384759423686,"x":2.140192379711843,"y":-2.140192379711843});
    child["outline"] = json!({"enabled":false,"color":"#000000","width":1.4267949198078953});
    child["blur"] = json!(11.414359358463162);
    child["bubbleFill"] = json!("#ffffff");
    child["bubbleBorder"] = json!("#ef3340");
    child["bubbleBorderWidth"] = json!(3.566987299519738);
    v["objects"] = json!([image, child]);

    let p = import(&v).unwrap();
    let mark = &p.document.objects[1];
    assert_eq!(mark.parent, Some(p.document.objects[0].id));
    assert_eq!(mark.style.width, 10);
    assert_eq!(mark.effects.shadow.blur, 6);
    assert_eq!(mark.effects.shadow.x, 3);
    assert_eq!(mark.effects.shadow.y, -3);
    assert_eq!(mark.rendering, PathRendering::DotNet);
    assert_eq!(
        p.document.parent_transform(mark.id).point(mark.points[0]),
        Point::new(5., 5.)
    );
}

#[test]
fn scaled_image_restores_owned_bubble_layout_effects_layer_and_round_trip() {
    let mut v = fixture();
    let scale = 57.07179679231581 / 80.;
    let mut image = object("image");
    image["id"] = json!("photo");
    image["assetId"] = json!("base");
    image["sourceCrop"] = json!({"x":0,"y":0,"width":80,"height":60});
    image["x1"] = json!(0);
    image["y1"] = json!(0);
    image["x2"] = json!(80. * scale);
    image["y2"] = json!(60. * scale);
    image["opacity"] = json!(1);
    image.as_object_mut().unwrap().remove("stroke");

    let mut bubble = object("bubble");
    bubble["owner"] = json!("photo");
    bubble["childId"] = json!("note");
    bubble.as_object_mut().unwrap().remove("x2");
    bubble.as_object_mut().unwrap().remove("y2");
    bubble.as_object_mut().unwrap().remove("stroke");
    bubble["x1"] = json!(10);
    bubble["y1"] = json!(8);
    bubble["size"] = json!(5. * scale);
    bubble["opacity"] = json!(0.75);
    bubble["text"] = json!("ไทย\nEnglish");
    bubble["bubbleWidth"] = json!(60. * scale);
    bubble["bubbleHeight"] = json!(35. * scale);
    bubble["bubbleFill"] = json!("#f0f1f2");
    bubble["bubbleBorder"] = json!("#112233");
    bubble["bubbleBorderWidth"] = json!(4. * scale);
    bubble["tail"] = json!({"x":20. * scale,"y":50. * scale});
    bubble["shadow"] =
        json!({"enabled":true,"color":"#405060","blur":6. * scale,"x":-3. * scale,"y":4. * scale});
    bubble["outline"] = json!({"enabled":true,"color":"#102030","width":3. * scale});
    v["objects"] = json!([image, bubble]);

    let p = import(&v).unwrap();
    let group = p.document.objects[0].id;
    let note = &p.document.objects[1];
    assert_eq!(note.parent, Some(group));
    assert_eq!(
        p.document.parent_transform(note.id).point(note.points[0]),
        Point::new(10., 8.)
    );
    assert_eq!(note.opacity, 75);
    assert_eq!(note.effects.shadow.blur, 6);
    assert_eq!(note.effects.shadow.x, -3);
    assert_eq!(note.effects.shadow.y, 4);
    assert_eq!(note.effects.outline.width, 3);
    let Content::Text {
        size, layout, tail, ..
    } = note.content
    else {
        panic!("expected editable owned bubble")
    };
    assert_eq!(size, 20);
    assert_eq!(tail, Some(Point::new(20., 50.)));
    let TextLayout::DotNet {
        source_size,
        bubble: Some(frame),
    } = layout
    else {
        panic!("expected retained .NET bubble layout")
    };
    assert_eq!(source_size, 5.);
    assert!((frame.width - 60.).abs() < 0.000001);
    assert!((frame.min_height - 35.).abs() < 0.000001);
    assert!((frame.border_width - 4.).abs() < 0.000001);

    let reopened =
        crate::project::decode(&crate::project::encode(&p.frame, &p.document).unwrap()).unwrap();
    assert_eq!(reopened.document.objects, p.document.objects);
    assert_eq!(
        crate::render::flatten(&reopened.frame, &reopened.document).unwrap(),
        crate::render::flatten(&p.frame, &p.document).unwrap()
    );
}

#[test]
fn single_point_dotnet_pen_remains_a_visible_editable_dot() {
    let mut v = fixture();
    let mut pen = object("pen");
    pen["points"] = json!([{"x":20,"y":20}]);
    pen["opacity"] = json!(1);
    pen.as_object_mut().unwrap().remove("stroke");
    v["objects"] = json!([pen]);

    let p = import(&v).unwrap();
    assert_eq!(p.document.objects[0].points.len(), 1);
    assert_eq!(p.document.objects[0].rendering, PathRendering::DotNet);
    let output = crate::render::flatten(&p.frame, &p.document).unwrap();
    let center = ((20 * output.width + 20) * 4) as usize;
    assert_eq!(&output.bgra[center..center + 4], &[64, 51, 239, 255]);
    let reopened =
        crate::project::decode(&crate::project::encode(&p.frame, &p.document).unwrap()).unwrap();
    assert_eq!(reopened.document.objects, p.document.objects);
}

#[test]
fn numbered_marker_and_blur_import_as_editable_dotnet_objects() {
    let mut v = fixture();
    let mut number = object("number");
    number.as_object_mut().unwrap().remove("x2");
    number.as_object_mut().unwrap().remove("y2");
    number.as_object_mut().unwrap().remove("stroke");
    number["number"] = json!(12);
    number["size"] = json!(10);
    number["opacity"] = json!(0.75);
    number["shadow"] = json!({"enabled":true,"color":"#123456","blur":6,"x":-3,"y":4});

    let mut blur = object("blur");
    blur.as_object_mut().unwrap().remove("stroke");
    blur["blur"] = json!(64);
    blur["opacity"] = json!(0.25);
    blur["shadow"] = json!({"enabled":true,"color":"#654321","blur":4,"x":2,"y":-2});
    v["objects"] = json!([number, blur]);

    let p = import(&v).unwrap();
    assert_eq!(p.document.objects.len(), 2);
    let step = &p.document.objects[0];
    assert_eq!(step.tool, Tool::Step);
    assert_eq!(step.rendering, PathRendering::DotNet);
    assert_eq!(step.points.as_ref(), &vec![Point::new(5., 10.)]);
    assert_eq!(step.style.width, 10);
    assert_eq!(step.opacity, 75);
    assert!(step.effects.shadow.enabled);
    let Content::Step {
        number,
        diameter,
        mask,
    } = &step.content
    else {
        panic!("expected imported step")
    };
    assert_eq!((*number, *diameter), (12, 64));
    assert!(mask.width > 0 && mask.height > 0);

    let blur = &p.document.objects[1];
    assert_eq!(blur.tool, Tool::Blur);
    assert_eq!(blur.rendering, PathRendering::DotNet);
    assert_eq!(blur.opacity, 25);
    assert!(blur.effects.shadow.enabled);
    assert!(matches!(blur.content, Content::Blur { strength: 64 }));
}

#[test]
fn imported_dotnet_step_edits_source_size_and_recomputes_its_circle() {
    let mut v = fixture();
    let mut number = object("number");
    number.as_object_mut().unwrap().remove("x2");
    number.as_object_mut().unwrap().remove("y2");
    number.as_object_mut().unwrap().remove("stroke");
    number["number"] = json!(12);
    number["size"] = json!(10);
    number["opacity"] = json!(1);
    v["objects"] = json!([number]);

    let mut p = import(&v).unwrap();
    let id = p.document.objects[0].id;
    p.document.selected = Some(id);
    assert_eq!(p.document.objects[0].parameter(), 10);
    assert!(p.document.set_parameter(12).unwrap());
    assert_eq!(p.document.objects[0].style.width, 12);
    assert!(matches!(
        p.document.objects[0].content,
        Content::Step {
            number: 12,
            diameter: 77,
            ..
        }
    ));
    assert!(p.document.undo());
    assert_eq!(p.document.objects[0].parameter(), 10);
}

#[test]
fn dotnet_step_renders_the_source_white_ring() {
    let mut v = fixture();
    let mut number = object("number");
    number.as_object_mut().unwrap().remove("x2");
    number.as_object_mut().unwrap().remove("y2");
    number.as_object_mut().unwrap().remove("stroke");
    number["number"] = json!(1);
    number["size"] = json!(5);
    number["opacity"] = json!(1);
    number["x1"] = json!(40);
    number["y1"] = json!(30);
    v["objects"] = json!([number]);

    let p = import(&v).unwrap();
    let output = crate::render::flatten(&p.frame, &p.document).unwrap();
    let edge = ((30 * output.width + 55) * 4) as usize;
    let ring = &output.bgra[edge..edge + 4];
    assert!(ring[0] > 64 && ring[1] > 51 && ring[2] > 239);
    assert_eq!(ring[3], 255);
}

#[test]
fn imported_blur_edits_the_full_source_range_and_undoes() {
    let mut v = fixture();
    let mut blur = object("blur");
    blur.as_object_mut().unwrap().remove("stroke");
    blur["blur"] = json!(64);
    blur["opacity"] = json!(1);
    v["objects"] = json!([blur]);

    let mut p = import(&v).unwrap();
    let id = p.document.objects[0].id;
    p.document.selected = Some(id);
    assert_eq!(p.document.objects[0].parameter(), 64);
    assert!(p.document.set_parameter(1).unwrap());
    assert!(matches!(
        p.document.objects[0].content,
        Content::Blur { strength: 1 }
    ));
    assert!(p.document.undo());
    assert_eq!(p.document.objects[0].parameter(), 64);
}

#[test]
fn zero_opacity_imported_blur_keeps_the_beneath_pixels() {
    let mut v = fixture();
    let mut pixels = vec![255; 80 * 60 * 4];
    for y in 0..60usize {
        for x in 0..40usize {
            let i = (y * 80 + x) * 4;
            pixels[i..i + 3].fill(0);
        }
    }
    let source = Frame::new(80, 60, pixels).unwrap();
    let mut png = vec![];
    source.write_png(&mut png).unwrap();
    v["assets"][0]["dataUrl"] = json!(format!("data:image/png;base64,{}", STANDARD.encode(png)));
    let mut blur = object("blur");
    blur.as_object_mut().unwrap().remove("stroke");
    blur["x1"] = json!(25);
    blur["y1"] = json!(5);
    blur["x2"] = json!(55);
    blur["y2"] = json!(55);
    blur["blur"] = json!(20);
    blur["opacity"] = json!(0);
    blur["shadow"] = json!({"enabled":true,"color":"#654321","blur":6,"x":3,"y":3});
    v["objects"] = json!([blur]);

    let mut p = import(&v).unwrap();
    assert_eq!(
        *crate::render::flatten(&p.frame, &p.document).unwrap(),
        source
    );
    p.document.selected = Some(p.document.objects[0].id);
    assert!(p.document.set_opacity(100));
    let full = crate::render::flatten(&p.frame, &p.document).unwrap();
    assert_ne!(*full, source);
    assert!(p.document.set_opacity(50));
    let half = crate::render::flatten(&p.frame, &p.document).unwrap();
    let i = ((30 * source.width + 39) * 4) as usize;
    assert_ne!(&full.bgra[i..i + 3], &source.bgra[i..i + 3]);
    for c in 0..3 {
        let expected = (full.bgra[i + c] as u32 + source.bgra[i + c] as u32 + 1) / 2;
        assert!((half.bgra[i + c] as i32 - expected as i32).abs() <= 1);
    }
}

#[test]
fn scaled_image_restores_owned_step_blur_geometry_effects_and_round_trip() {
    let mut v = fixture();
    let scale = 57.07179679231581 / 80.;
    let mut image = object("image");
    image["id"] = json!("photo");
    image["assetId"] = json!("base");
    image["sourceCrop"] = json!({"x":0,"y":0,"width":80,"height":60});
    image["x1"] = json!(0);
    image["y1"] = json!(0);
    image["x2"] = json!(80. * scale);
    image["y2"] = json!(60. * scale);
    image["opacity"] = json!(1);
    image.as_object_mut().unwrap().remove("stroke");

    let mut number = object("number");
    number["owner"] = json!("photo");
    number["childId"] = json!("step");
    number.as_object_mut().unwrap().remove("x2");
    number.as_object_mut().unwrap().remove("y2");
    number.as_object_mut().unwrap().remove("stroke");
    number["number"] = json!(12);
    number["size"] = json!(10. * scale);
    number["opacity"] = json!(0.75);
    number["shadow"] =
        json!({"enabled":true,"color":"#123456","blur":6. * scale,"x":-3. * scale,"y":4. * scale});

    let mut blur = object("blur");
    blur["owner"] = json!("photo");
    blur["childId"] = json!("blur");
    blur.as_object_mut().unwrap().remove("stroke");
    blur["size"] = json!(5. * scale);
    blur["blur"] = json!(64. * scale);
    blur["opacity"] = json!(0.25);
    blur["shadow"] =
        json!({"enabled":true,"color":"#654321","blur":4. * scale,"x":2. * scale,"y":-2. * scale});
    v["objects"] = json!([image, number, blur]);

    let p = import(&v).unwrap();
    let group = p.document.objects[0].id;
    let step = &p.document.objects[1];
    assert_eq!(step.parent, Some(group));
    assert_eq!(step.style.width, 10);
    assert_eq!(step.effects.shadow.blur, 6);
    assert_eq!(step.effects.shadow.x, -3);
    assert_eq!(step.effects.shadow.y, 4);
    assert_eq!(
        p.document.parent_transform(step.id).point(step.points[0]),
        Point::new(5., 10.)
    );
    assert!(matches!(
        step.content,
        Content::Step {
            number: 12,
            diameter: 64,
            ..
        }
    ));

    let blur = &p.document.objects[2];
    assert_eq!(blur.parent, Some(group));
    assert_eq!(blur.opacity, 25);
    assert_eq!(blur.effects.shadow.blur, 4);
    assert!(matches!(blur.content, Content::Blur { strength: 64 }));
    assert_eq!(
        p.document.parent_transform(blur.id).point(blur.points[1]),
        Point::new(65., 40.)
    );

    let reopened =
        crate::project::decode(&crate::project::encode(&p.frame, &p.document).unwrap()).unwrap();
    assert_eq!(reopened.document.objects, p.document.objects);
}

#[test]
fn enlarged_image_validates_blur_after_restoring_local_scale() {
    let mut v = fixture();
    let mut image = object("image");
    image["id"] = json!("photo");
    image["assetId"] = json!("base");
    image["sourceCrop"] = json!({"x":0,"y":0,"width":80,"height":60});
    image["x1"] = json!(0);
    image["y1"] = json!(0);
    image["x2"] = json!(160);
    image["y2"] = json!(120);
    image["opacity"] = json!(1);
    image.as_object_mut().unwrap().remove("stroke");

    let mut blur = object("blur");
    blur["owner"] = json!("photo");
    blur["childId"] = json!("blur");
    blur.as_object_mut().unwrap().remove("stroke");
    blur["size"] = json!(10);
    blur["blur"] = json!(128);
    blur["opacity"] = json!(1);
    v["objects"] = json!([image, blur]);

    let p = import(&v).unwrap();
    assert!(matches!(
        p.document.objects[1].content,
        Content::Blur { strength: 64 }
    ));
}

#[test]
fn largest_dotnet_step_keeps_the_source_font_formula() {
    assert_eq!(crate::glyphs::dotnet_step_font_size(1_000_000, 32), 143);
    let mask = crate::glyphs::dotnet_step(1_000_000, 32).unwrap();
    assert!(mask.width > 0 && mask.height > 0);
}

#[test]
fn invalid_step_and_blur_values_reject_the_entire_file() {
    for value in [json!(0), json!(1_000_001)] {
        let mut v = fixture();
        let mut number = object("number");
        number.as_object_mut().unwrap().remove("x2");
        number.as_object_mut().unwrap().remove("y2");
        number.as_object_mut().unwrap().remove("stroke");
        number["number"] = value;
        v["objects"] = json!([object("box"), number]);
        assert!(import(&v).is_err());
    }
    for value in [None, Some(json!(0)), Some(json!(65))] {
        let mut v = fixture();
        let mut blur = object("blur");
        blur.as_object_mut().unwrap().remove("stroke");
        if let Some(value) = value {
            blur["blur"] = value;
        }
        v["objects"] = json!([object("box"), blur]);
        assert!(import(&v).is_err());
    }
    let mut v = fixture();
    let mut blur = object("blur");
    blur.as_object_mut().unwrap().remove("stroke");
    blur["blur"] = json!(8);
    blur["x2"] = blur["x1"].clone();
    v["objects"] = json!([object("box"), blur]);
    assert!(import(&v).is_err());
}

#[test]
fn unsupported_tool_diagnostic_no_longer_lists_step_or_blur() {
    let mut v = fixture();
    v["objects"] = json!([object("future-tool")]);
    let err = match decode(&serde_json::to_vec(&v).unwrap(), 32 * 1024 * 1024) {
        Err(err) => err,
        Ok(_) => panic!("unknown tool must reject the complete file"),
    };
    let english = err.message(false);
    assert!(!english.contains("Text"), "{english}");
    assert!(!english.contains("speech bubbles"), "{english}");
    assert!(!english.contains("step numbers"), "{english}");
    assert!(!english.contains("blur"), "{english}");
    let thai = err.message(true);
    assert!(!thai.contains("เลขลำดับ"), "{thai}");
    assert!(!thai.contains("เบลอ"), "{thai}");
    assert!(!thai.contains("ช่องคำพูด"), "{thai}");
}
