use limivra_core::{
    adb::{parse_adb_devices_output, DeviceStatus},
    display::DisplayProber,
    ScrcpyVersion,
};

const LINE_LIMIT: usize = 16 * 1024;

#[test]
fn adb_parsing_keeps_complete_records_and_never_promotes_unknown_status() {
    let output = format!(
        "List of devices attached\nList of devices attached\n\n\
         device-1 device model:Pixel product:test transport_id:7\n\
         device-2 mystery transport_id:8 model:Unknown\n\
         truncated\n\
         replacement-� device model:Broken\n\
         {} device\n",
        "x".repeat(LINE_LIMIT + 1)
    );

    let devices = parse_adb_devices_output(&output);
    assert_eq!(devices.len(), 2, "malformed rows must be dropped");
    assert_eq!(devices[0].serial, "device-1");
    assert_eq!(devices[0].status, DeviceStatus::Device);
    assert_eq!(devices[0].model.as_deref(), Some("Pixel"));
    assert_eq!(devices[1].serial, "device-2");
    assert_eq!(devices[1].status, DeviceStatus::Unknown);
    assert!(!devices[1].is_ready());
}

#[test]
fn dumpsys_parser_rejects_truncated_incomplete_replacement_and_huge_records() {
    let oversized = format!(
        "DisplayDeviceInfo{{\"{}\": uniqueId=\"virtual:8\", 640 x 480}}",
        "x".repeat(LINE_LIMIT)
    );
    let output = format!(
        "Display Id=4\n\
         DisplayDeviceInfo{{\"valid\": uniqueId=\"virtual:4\", 1280 x 720, density 240, type VIRTUAL}}\n\
         DisplayDeviceInfo{{\"truncated\": uniqueId=\"virtual:5\", 640 x 480\n\
         DisplayDeviceInfo{{\"no-bounds\": uniqueId=\"virtual:6\", density 240, type VIRTUAL}}\n\
         DisplayDeviceInfo{{\"replacement-�\": uniqueId=\"virtual:7\", 640 x 480, density 240}}\n\
         {oversized}\n"
    );

    let displays = DisplayProber::parse_dumpsys_display_output(&output);
    assert_eq!(displays.len(), 1, "only the complete display is trusted");
    assert_eq!(displays[0].id, 4);
    assert_eq!(displays[0].name, "valid");
    assert_eq!((displays[0].width, displays[0].height), (1280, 720));
}

#[test]
fn scrcpy_display_list_rejects_missing_or_invalid_dimensions_and_huge_lines() {
    let oversized = format!("--display-id=4 640x480 # {}", "x".repeat(LINE_LIMIT));
    let output = format!(
        "--display-id=0 1080x2400\n\
         --display-id=1\n\
         --display-id=2 not-a-rectangle\n\
         --display-id=3 0x0\n\
         {oversized}\n"
    );

    let displays = DisplayProber::parse_scrcpy_list_displays_output(&output);
    assert_eq!(displays.len(), 1);
    assert_eq!(displays[0].id, 0);
    assert_eq!((displays[0].width, displays[0].height), (1080, 2400));
}

#[test]
fn scrcpy_version_parser_fails_closed_on_malformed_or_oversized_first_line() {
    for malformed in [
        "scrcpy 3.x",
        "scrcpy 3.2.1.4",
        "scrcpy �.2.1",
        &format!("scrcpy 3.2.1 {}", "x".repeat(LINE_LIMIT)),
    ] {
        assert!(
            ScrcpyVersion::parse(malformed).is_none(),
            "malformed version was accepted: {:?}",
            &malformed[..malformed.len().min(64)]
        );
    }

    assert_eq!(
        ScrcpyVersion::parse("scrcpy 3.2.1 <https://example.invalid>\nignored"),
        Some(ScrcpyVersion {
            major: 3,
            minor: 2,
            patch: Some(1),
            raw: "3.2.1".into(),
        })
    );
}

#[test]
fn malformed_inputs_never_panic() {
    let cases = ["", "\n", "{", "Display Id=-1", "--display-id=NaN", "scrcpy"];
    for case in cases {
        assert!(std::panic::catch_unwind(|| parse_adb_devices_output(case)).is_ok());
        assert!(
            std::panic::catch_unwind(|| { DisplayProber::parse_dumpsys_display_output(case) })
                .is_ok()
        );
        assert!(std::panic::catch_unwind(|| {
            DisplayProber::parse_scrcpy_list_displays_output(case)
        })
        .is_ok());
        assert!(std::panic::catch_unwind(|| ScrcpyVersion::parse(case)).is_ok());
    }
}
