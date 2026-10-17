// use ash::{Entry, vk};
// fn main() {
//     unsafe {
//         // 1. Load the Vulkan library
//         let entry = Entry::load().expect("Vulkan not found");

//         // 2. Fill the settings form
//         let create_info = vk::InstanceCreateInfo::default();

//         // 3. Create the instance
//         let instance = entry
//             .create_instance(&create_info, None)
//             .expect("Instance failed");

//         println!("Vulkan instance created!");

//         // 4. Destroy it ourselves
//         instance.destroy_instance(None);
//     }
// }
use ash::{Entry, vk};

fn main() {
    unsafe {
        let entry = Entry::load().expect("Vulkan not found");
        let create_info = vk::InstanceCreateInfo::default();
        let instance = entry.create_instance(&create_info, None).unwrap();

        // Ask for the list of GPUs
        let gpus = instance.enumerate_physical_devices().unwrap();
        println!("Found {} GPU(s)", gpus.len());

        for gpu in gpus {
            // Ask what this GPU is
            let props = instance.get_physical_device_properties(gpu);
            println!("\nName: {:?}", props.device_name_as_c_str().unwrap());
            println!("Type: {:?}", props.device_type);

            // Ask what kinds of inboxes it has
            let families = instance.get_physical_device_queue_family_properties(gpu);
            for (i, f) in families.iter().enumerate() {
                println!(
                    "  family {}: {} queues | graphics: {} | compute: {} | transfer: {}",
                    i,
                    f.queue_count,
                    f.queue_flags.contains(vk::QueueFlags::GRAPHICS),
                    f.queue_flags.contains(vk::QueueFlags::COMPUTE),
                    f.queue_flags.contains(vk::QueueFlags::TRANSFER),
                );
            }
        }

        instance.destroy_instance(None);
    }
}
