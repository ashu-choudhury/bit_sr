//! Live Accessibility Tree Probe
//! Queries the real live focused element and its surrounding tree from the Windows OS.

use bit_sr_platform_windows::WindowsPlatform;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================================");
    println!("     bit_sr: Live Windows Accessibility Tree Inspector      ");
    println!("============================================================");

    let (tx, _rx) = crossbeam_channel::unbounded();
    let platform = WindowsPlatform::start(tx)?;

    println!("Querying live focused element from Windows UI Automation MTA...");
    if let Some(uia) = platform.uia() {
        match uia.get_focused_element() {
            Ok(element) => {
                let node = element.to_accessible_node();
                println!("\n[LIVE FOCUSED ELEMENT]");
                println!("  Node ID:         {:?}", node.id);
                println!("  Role:            {:?} (\"{}\")", node.role, node.role.display_name());
                println!("  Name:            {:?}", node.name);
                println!("  Class Name:      {:?}", node.class_name);
                println!("  Automation ID:   {:?}", node.automation_id);
                println!("  Process ID:      {:?}", node.process_id);
                println!("  Node ID:         0x{:X}", node.id.0);
                println!("  States:          {:?} ({:?})", node.states, node.states.to_speech_strings());
                if let Some(ref rect) = node.bounds {
                    println!("  Bounding Rect:   [x={:.0}, y={:.0}, w={:.0}, h={:.0}]", rect.left, rect.top, rect.width, rect.height);
                }
                let pos = &node.position_info;
                println!(
                    "  Position In Set: {:?} of {:?} (level {:?})",
                    pos.position_in_set, pos.size_of_set, pos.level
                );

                // Check ControlView navigation (Parent and First Child)
                if let Ok(nav) = uia.control_view_navigator() {
                    println!("\n[STRUCTURAL CONTROL-VIEW NAVIGATION]");
                    if let Some(parent) = nav.get_parent(element.raw()) {
                        let p_node = bit_sr_platform_windows::uia::UiaElement::new(parent).to_accessible_node();
                        println!("  Parent Node:     Role={:?} Name={:?} Class={:?}", p_node.role, p_node.name, p_node.class_name);
                    } else {
                        println!("  Parent Node:     None (Root or Desktop element)");
                    }

                    if let Some(child) = nav.get_first_child(element.raw()) {
                        let c_node = bit_sr_platform_windows::uia::UiaElement::new(child).to_accessible_node();
                        println!("  First Child:     Role={:?} Name={:?} Class={:?}", c_node.role, c_node.name, c_node.class_name);
                    } else {
                        println!("  First Child:     None (Leaf element)");
                    }
                }
            }
            Err(e) => {
                println!("Failed to query focused element: {:?}", e);
            }
        }
    }

    println!("\nLive OS query completed successfully.");
    Ok(())
}
