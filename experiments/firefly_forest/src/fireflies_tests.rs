use super::*;
use glam::{Mat4, Vec3};

#[test]
fn initial_population_is_finite_bounded_and_has_varied_cycles() {
    let settings = Settings::default();
    let particles = initial_particles(&settings, PARTICLE_COUNT);
    assert_eq!(particles.len(), PARTICLE_COUNT as usize);
    for p in &particles {
        assert!(bytemuck::cast_slice::<_, f32>(std::slice::from_ref(p))
            .iter()
            .all(|v| v.is_finite()));
        for axis in 0..3 {
            assert!(p.position_phase[axis] >= settings.min[axis] + BODY_RADIUS);
            assert!(p.position_phase[axis] <= settings.max[axis] - BODY_RADIUS);
        }
        assert!((0.0..1.0).contains(&p.position_phase[3]));
        assert!((0.15..0.3).contains(&p.velocity_frequency[3]));
    }
    assert!(particles.iter().any(|p| p.position_phase[3] < 0.1));
    assert!(particles.iter().any(|p| p.position_phase[3] > 0.9));
    assert!(particles.iter().any(|p| p.velocity_frequency[3] < 0.18));
    assert!(particles.iter().any(|p| p.velocity_frequency[3] > 0.26));
    assert!(particles.iter().any(|p| p.position_phase[0] < -8.));
    assert!(particles.iter().any(|p| p.position_phase[0] > 8.));
    assert!(particles.iter().any(|p| p.position_phase[2] < -20.));
    assert!(particles.iter().any(|p| p.position_phase[2] > 4.));
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_close_particles_have_independent_wandering_at_low_speed() {
    let device = device();
    let settings = Settings {
        separation: 0.,
        cohesion: 0.,
        alignment: 0.,
        phase_jitter: 0.,
        ..Default::default()
    };
    let mut system = FireflySystem::with_particles(&device, &[particle(0., 0.2); 2], settings);
    system.set_rates(0.05, 0.).unwrap();
    let output = step(&device, &mut system, 0.1);
    let a = Vec3::from_slice(&output[0].velocity_frequency);
    let b = Vec3::from_slice(&output[1].velocity_frequency);
    assert!(
        a.distance(b) > 1e-5,
        "nearby particles must not share one wind direction"
    );
    system.set_rates(0., 0.).unwrap();
    assert_eq!(step(&device, &mut system, 0.1), output);
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_slow_blinking_retains_phase_diversity_despite_neighbor_sync() {
    let device = device();
    let input = [particle(0., 0.2); 64];
    let mut system = FireflySystem::with_particles(&device, &input, Settings::default());
    system.set_rates(0., 0.05).unwrap();
    let mut output = Vec::new();
    for _ in 0..200 {
        output = step(&device, &mut system, 0.1);
    }
    let min = output
        .iter()
        .map(|p| p.position_phase[3])
        .fold(1., f32::min);
    let max = output
        .iter()
        .map(|p| p.position_phase[3])
        .fold(0., f32::max);
    assert!(
        max - min > 0.04,
        "phase spread at 0.05× is only {}",
        max - min
    );
    let mean = output.iter().map(|p| p.position_phase[3]).sum::<f32>() / 64.;
    assert!(
        (0.4..0.44).contains(&mean),
        "average frequency must stay slow: {mean}"
    );
    system.set_rates(0., 0.).unwrap();
    assert_eq!(step(&device, &mut system, 0.1), output);
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_rates_independently_slow_and_pause_motion_and_blinking() {
    let device = device();
    let settings = Settings {
        noise_strength: 0.,
        phase_jitter: 0.,
        drag: 0.,
        ..Default::default()
    };
    let mut input = particle(0., 0.2);
    input.velocity_frequency[0] = 0.1;
    let run = |motion, blink| {
        let mut system = FireflySystem::with_particles(&device, &[input], settings);
        system.set_rates(motion, blink).unwrap();
        step(&device, &mut system, 0.1)[0]
    };
    let normal = run(1., 1.);
    let slow_motion = run(0.5, 1.);
    let slow_blink = run(1., 0.5);
    assert!((normal.position_phase[0] - 0.01).abs() < 1e-5);
    assert!((slow_motion.position_phase[0] - 0.005).abs() < 1e-5);
    assert!((slow_motion.position_phase[3] - normal.position_phase[3]).abs() < 1e-5);
    assert!((slow_blink.position_phase[0] - normal.position_phase[0]).abs() < 1e-5);
    assert!((slow_blink.position_phase[3] - 0.211).abs() < 1e-5);
    assert_eq!(run(0., 0.), input);
    assert_eq!(run(2., 2.), normal);
    assert_eq!(run(-1., -1.), input);

    let mut system = FireflySystem::with_particles(&device, &[input], settings);
    system.set_rates(0.5, 0.).unwrap();
    assert!(system.set_rates(f32::NAN, 1.).is_err());
    let output = step(&device, &mut system, 0.1)[0];
    assert!((output.position_phase[0] - 0.005).abs() < 1e-5);
    assert_eq!(output.position_phase[3], input.position_phase[3]);
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_drag_smoothly_boosts_motion_and_restores_configured_rate() {
    let device = device();
    let settings = Settings {
        noise_strength: 0.,
        phase_jitter: 0.,
        drag: 0.,
        ..Default::default()
    };
    let mut input = particle(0., 0.2);
    input.velocity_frequency[0] = 0.1;
    let mut system = FireflySystem::with_particles(&device, &[input], settings);
    system.set_rates(0.27, 0.).unwrap();
    system.set_render_mode(RenderMode::Drag);
    let mut x = 0.;
    let mut advance = |system: &mut FireflySystem| {
        let output = step(&device, system, 0.05)[0];
        assert_eq!(output.position_phase[3], input.position_phase[3]);
        let rate = (output.position_phase[0] - x) / 0.005;
        x = output.position_phase[0];
        rate
    };
    assert!((advance(&mut system) - 0.27).abs() < 1e-5);
    system.set_dragging(true);
    let first = advance(&mut system);
    assert!(
        (0.27..1.).contains(&first),
        "first drag step must ease in: {first}"
    );
    let mut rate = first;
    for _ in 1..5 {
        let next = advance(&mut system);
        assert!(next > rate);
        rate = next;
    }
    assert!((rate - 1.).abs() < 1e-5);
    let time = system.time;
    advance(&mut system);
    assert!((system.time - time - 0.05).abs() < 1e-5);
    assert_eq!(system.rates, [0.27, 0.]);

    system.set_dragging(false);
    let released = advance(&mut system);
    assert!((0.27..1.).contains(&released));
    system.set_dragging(true);
    assert!(advance(&mut system) > released);
    system.set_dragging(false);
    rate = 1.;
    for _ in 0..14 {
        let next = advance(&mut system);
        assert!(next <= rate + 1e-5);
        rate = next;
    }
    assert!((rate - 0.27).abs() < 1e-5);
    system.set_rates(0., 0.).unwrap();
    assert!(advance(&mut system).abs() < 1e-5);
    system.set_dragging(true);
    let from_paused = advance(&mut system);
    assert!(from_paused > 0. && from_paused < 1.);
    for mode in [RenderMode::Particles, RenderMode::Trails] {
        system.set_render_mode(mode);
        assert!(advance(&mut system).abs() < 1e-5);
    }
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_phase_randomness_is_individual_bounded_and_pauses_with_frequency() {
    let device = device();
    let input = [particle(-2., 0.2), particle(2., 0.2)];
    let mut system = FireflySystem::with_particles(&device, &input, Settings::default());
    let output = step(&device, &mut system, 0.1);
    assert!((output[0].position_phase[3] - output[1].position_phase[3]).abs() > 1e-6);
    for p in &output {
        assert!((0.2135..=0.2265).contains(&p.position_phase[3]));
    }
    system.set_rates(1., 0.).unwrap();
    let paused = step(&device, &mut system, 0.1);
    for (before, after) in output.iter().zip(paused) {
        assert_eq!(before.position_phase[3], after.position_phase[3]);
    }
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_flash_duration_is_fixed_at_different_blink_rates() {
    let device = device();
    let settings = Settings {
        phase_jitter: 0.,
        ..Default::default()
    };
    for rate in [1., 0.5, 0.1] {
        // The next cycle wraps 20 ms into the first 30 ms frame at every rate.
        let input = particle(0., 1. - 0.02 * 0.22 * rate);
        let mut system = FireflySystem::with_particles(&device, &[input], settings);
        system.set_flash_duration(0.1).unwrap();
        system.set_rates(0., rate).unwrap();
        let started = step(&device, &mut system, 0.03)[0];
        assert!((started.flash[0] - 0.01).abs() < 2e-5);
        assert!(started.flash[1] > 0.);
        let peak = step(&device, &mut system, 0.04)[0];
        assert!((peak.flash[0] - 0.05).abs() < 2e-5);
        assert!(peak.flash[1] > 0.999);
        let fading = step(&device, &mut system, 0.049)[0];
        assert!(fading.flash[1] > 0.);
        let ended = step(&device, &mut system, 0.002)[0];
        assert_eq!(
            ended.flash[1], 0.,
            "pulse must end after 100 ms at rate {rate}"
        );
    }
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_changing_frequency_mid_flash_does_not_stretch_or_freeze_it() {
    let device = device();
    let settings = Settings {
        phase_jitter: 0.,
        ..Default::default()
    };
    let input = particle(0., 1. - 0.01 * 0.22);
    let mut system = FireflySystem::with_particles(&device, &[input], settings);
    system.set_flash_duration(0.1).unwrap();
    system.set_rates(0., 1.).unwrap();
    let started = step(&device, &mut system, 0.03)[0];
    assert!((started.flash[0] - 0.02).abs() < 2e-5);
    system.set_rates(0., 0.1).unwrap();
    let slowed = step(&device, &mut system, 0.04)[0];
    assert!((slowed.flash[0] - 0.06).abs() < 2e-5);
    assert!(slowed.flash[1] > 0.8);
    system.set_rates(0., 0.).unwrap();
    let ended = step(&device, &mut system, 0.05)[0];
    assert_eq!(ended.flash[1], 0.);
    assert_eq!(ended.position_phase[3], slowed.position_phase[3]);
    assert_eq!(step(&device, &mut system, 0.1)[0], ended);
    system.set_rates(0., 1.).unwrap();
    let resumed = step(&device, &mut system, 0.05)[0];
    assert!(resumed.position_phase[3] > ended.position_phase[3]);
    assert_eq!(
        resumed.flash[1], 0.,
        "resuming must wait for the next cycle"
    );
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_selected_flash_duration_is_independent_of_blink_frequency() {
    let device = device();
    let settings = Settings {
        phase_jitter: 0.,
        ..Default::default()
    };
    for duration in [0.1, 0.5, 2.] {
        for rate in [1., 0.25] {
            let input = particle(0., 1. - 0.02 * 0.22 * rate);
            let mut system = FireflySystem::with_particles(&device, &[input], settings);
            system.set_flash_duration(duration).unwrap();
            system.set_rates(0., rate).unwrap();
            let started = step(&device, &mut system, 0.03)[0];
            assert!((started.flash[0] - 0.01).abs() < 2e-5);
            assert_eq!(started.flash[2], duration);
            for frame in 1..=(duration / 0.05).round() as u32 {
                let output = step(&device, &mut system, 0.05)[0];
                let elapsed = 0.01 + frame as f32 * 0.05;
                assert_eq!(
                    output.flash[1] > 0.,
                    elapsed < duration,
                    "duration={duration}, rate={rate}, elapsed={elapsed}"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_duration_changes_apply_to_next_flash_without_relighting_or_stretching() {
    let device = device();
    let settings = Settings {
        phase_jitter: 0.,
        ..Default::default()
    };
    let mut input = particle(0., 0.99);
    input.velocity_frequency[3] = 0.5;
    let mut system = FireflySystem::with_particles(&device, &[input], settings);
    assert_eq!(system.flash_duration, 1.5);
    system.set_rates(0., 1.).unwrap();
    system.set_flash_duration(0.).unwrap(); // Clamp to the 0.1 s minimum.
    assert!(system.set_flash_duration(f32::NAN).is_err());
    let started = step(&device, &mut system, 0.03)[0];
    assert_eq!(started.flash[2], 0.1);
    system.set_flash_duration(10.).unwrap(); // Clamp to the 2 s maximum.
    assert!(system.set_flash_duration(f32::INFINITY).is_err());
    let peak = step(&device, &mut system, 0.04)[0];
    assert!(peak.flash[1] > 0.999);
    assert_eq!(
        peak.flash[2], 0.1,
        "keep the active flash's original duration"
    );
    let mut previous = step(&device, &mut system, 0.06)[0];
    assert_eq!(previous.flash[1], 0.);
    for _ in 0..20 {
        let output = step(&device, &mut system, 0.1)[0];
        if output.position_phase[3] < previous.position_phase[3] {
            assert_eq!(output.flash[2], 2.);
            assert!(output.flash[1] > 0.);
            return;
        }
        assert_eq!(output.flash[1], 0., "finished flashes must not relight");
        previous = output;
    }
    panic!("expected the next flash to use the selected duration");
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_slower_frequency_only_increases_the_interval_between_flashes() {
    let device = device();
    let settings = Settings {
        phase_jitter: 0.,
        ..Default::default()
    };
    for (rate, expected_interval) in [(1., 2.), (0.5, 4.)] {
        let mut input = particle(0., 0.99);
        input.velocity_frequency[3] = 0.5;
        let mut system = FireflySystem::with_particles(&device, &[input], settings);
        system.set_rates(0., rate).unwrap();
        let mut last_phase = input.position_phase[3];
        let mut flashes = Vec::new();
        for frame in 1..=85 {
            let output = step(&device, &mut system, 0.05)[0];
            if output.position_phase[3] < last_phase {
                flashes.push(frame as f32 * 0.05 - output.flash[0]);
                assert!(output.flash[1] > 0.);
            }
            last_phase = output.position_phase[3];
        }
        assert!(flashes.len() >= 2);
        assert!((flashes[1] - flashes[0] - expected_interval).abs() < 1e-4);
    }
}

#[test]
fn firefly_shaders_validate() {
    for source in [
        simulation_shader(),
        draw_shader(),
        trails::record_shader(),
        trails::draw_shader(false),
        trails::draw_shader(true),
    ] {
        let module = naga::front::wgsl::parse_str(&source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
    }
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_trail_history_supports_maximum_population() {
    let device = device();
    let mut system = FireflySystem::new(&device);
    system.set_count(MAX_PARTICLE_COUNT);
    let output = step(&device, &mut system, 0.);
    assert_eq!(output.len(), MAX_PARTICLE_COUNT as usize);
    assert!(output
        .iter()
        .all(|p| p.position_phase.iter().all(|v| v.is_finite())));
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_trail_history_wraps_and_resets_without_stale_positions() {
    let device = device();
    let settings = Settings {
        noise_strength: 0.,
        phase_jitter: 0.,
        separation: 0.,
        cohesion: 0.,
        alignment: 0.,
        drag: 0.,
        ..Default::default()
    };
    let mut moving = particle(0., 0.2);
    moving.velocity_frequency[0] = 0.1;
    let mut system = FireflySystem::with_particles(&device, &[moving, particle(1., 0.2)], settings);
    system.count = 1;
    let history = |system: &FireflySystem| -> Vec<[f32; 4]> {
        system
            .trails
            .history
            .iter()
            .flat_map(|chunk| {
                bytemuck::cast_slice::<_, [f32; 4]>(&read_buffer(
                    &device,
                    chunk.gpu(),
                    chunk.size(),
                ))
                .to_vec()
            })
            .collect()
    };
    step(&device, &mut system, 0.025);
    for position in history(&system).iter().step_by(2) {
        assert!(
            (position[0] - 0.0025).abs() < 1e-5,
            "initialize every slot at the particle"
        );
    }
    for _ in 1..259 {
        step(&device, &mut system, 0.025);
    }
    let positions = history(&system);
    for age in 0..256 {
        let slot = (258 + 256 - age) % 256;
        assert!((positions[slot * 2][0] - (0.6475 - age as f32 * 0.0025)).abs() < 1e-5);
    }
    system.set_count(2);
    let output = step(&device, &mut system, 0.1);
    for (i, position) in history(&system).iter().enumerate() {
        assert_eq!(
            &position[..3],
            &output[i % 2].position_phase[..3],
            "count growth resets old paths"
        );
    }
    system.reset(&device, 42);
    let output = step(&device, &mut system, 0.);
    for (i, position) in history(&system).iter().enumerate() {
        assert_eq!(
            &position[..3],
            &output[i % 2].position_phase[..3],
            "seed reset must not bridge scenes"
        );
    }
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_trails_use_live_flashes_in_drag_mode_and_respect_depth() {
    let device = device();
    let mut camera = camera();
    camera.projection = Mat4::perspective_rh(0.4, 1., 0.02, 100.);
    let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
        label: Some("TrailTestColor"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: device.format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target = texture.create_view(&Default::default());
    let forest_depth = depth_view(&device, [64, 64]);
    let staging = device.gpu().create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 64 * 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let mut p = particle(-0.15, 0.2);
    p.position_phase[1] = 0.;
    p.velocity_frequency[0] = 0.2;
    let mut far = p;
    far.position_phase[2] = -4.;
    let mut system = FireflySystem::with_particles(
        &device,
        &[p, far],
        Settings {
            noise_strength: 0.,
            phase_jitter: 0.,
            drag: 0.,
            separation: 0.,
            cohesion: 0.,
            alignment: 0.,
            ..Default::default()
        },
    );
    system.count = 1;
    system.set_render_mode(RenderMode::Trails);
    system.set_size(3.).unwrap();
    system.set_rates(1., 0.).unwrap();
    system
        .set_appearance(&device, 0., 0., 0., &[1., 0., 0., 1., 0., 0., 1., 0., 0.])
        .unwrap();
    system.set_occlusion(&device, &forest_depth);
    for _ in 0..256 {
        step(&device, &mut system, 0.00625);
    }
    let render = |system: &mut FireflySystem, depth: f32| {
        system.update(&device, 0., &camera);
        let mut encoder = device.gpu().create_command_encoder(&Default::default());
        system.simulate(&mut encoder);
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &forest_depth,
                    stencil_ops: None,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(depth),
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                ..Default::default()
            });
        }
        system.draw_trails(&mut encoder, &target, None);
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(64),
                },
            },
            wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
        );
        device.queue().submit([encoder.finish()]);
        read_buffer(&device, &staging, 64 * 256)
    };
    let visible = render(&mut system, 1.);
    let red: Vec<_> = visible.chunks_exact(4).filter(|p| p[0] > 0).collect();
    assert!(red.len() > 5, "unlit fireflies still have visible trails");
    assert!(
        red.iter().all(|p| p[..] == [255, 0, 0, 255]),
        "solid red must replace blue without glow or blending"
    );
    let clip = camera.projection * glam::Vec4::new(0., 0., -2., 1.);
    assert!(
        render(&mut system, clip.z / clip.w)
            .chunks_exact(4)
            .all(|p| p[0] == 0),
        "forest in front hides the entire trail"
    );
    assert!(
        render(&mut system, 1.).chunks_exact(4).any(|p| p[0] > 0),
        "old trail depth must not persist across frames"
    );

    let mut live = step(&device, &mut system, 0.)[0];
    live.flash = [5., 0.5, 20., 0.];
    system.states[system.current].write(&device, &[live], 0);
    system.set_render_mode(RenderMode::Drag);
    assert!(
        render(&mut system, 1.).chunks_exact(4).all(|p| p[0] == 0),
        "idle must hide pre-drag history"
    );
    system.set_dragging(true);
    for _ in 0..60 {
        step(&device, &mut system, 0.00625);
    }
    let grown = render(&mut system, 1.);
    let red_sum = |pixels: &[u8]| pixels.chunks_exact(4).map(|p| u32::from(p[0])).sum::<u32>();
    assert!(red_sum(&grown) > 0, "drag grows a new trail");
    let flash_brightness = |system: &mut FireflySystem, intensity: f32| {
        let mut live = step(&device, system, 0.)[0];
        live.flash[1] = intensity;
        system.states[system.current].write(&device, &[live], 0);
        render(system, 1.)
            .chunks_exact(4)
            .map(|p| p[0])
            .max()
            .unwrap()
    };
    let held_dark = flash_brightness(&mut system, 0.);
    let held_half = flash_brightness(&mut system, 0.5);
    let held_bright = flash_brightness(&mut system, 1.);
    assert!(held_bright > 0);
    assert_eq!(
        held_dark, held_bright,
        "drag lights even particles between flashes"
    );
    assert_eq!(
        held_half, held_bright,
        "all held trails reach full brightness"
    );
    let head = step(&device, &mut system, 0.00625)[0].position_phase[0];
    system.set_dragging(false);
    for _ in 0..53 {
        step(&device, &mut system, 0.00625);
    }
    let shrinking = render(&mut system, 1.);
    assert!(red_sum(&shrinking) > 0 && red_sum(&shrinking) < red_sum(&grown));
    assert!(
        shrinking
            .chunks_exact(4)
            .any(|p| p[0] > 0 && p[0] < 255 && p[2] > 0),
        "trail fades over the forest color without additive glow"
    );
    let dark = flash_brightness(&mut system, 0.);
    let half = flash_brightness(&mut system, 0.5);
    let bright = flash_brightness(&mut system, 1.);
    assert!(
        dark < half && half < bright,
        "retracting trail follows the live flash"
    );
    assert_eq!(
        flash_brightness(&mut system, 0.),
        dark,
        "flash decay must not retain old brightness"
    );
    for _ in 0..60 {
        step(&device, &mut system, 0.00625);
    }
    assert_eq!(
        red_sum(&render(&mut system, 1.)),
        0,
        "release collapses the whole trail"
    );
    assert!(
        step(&device, &mut system, 0.)[0].position_phase[0] > head + 0.1,
        "particle keeps moving during retraction"
    );

    p.flash = [5., 0.5, 20., 0.];
    far.flash = p.flash;
    let mut overlap = |particles: &[Particle], retract: bool, regrab: bool| {
        system.count = particles.len() as u32;
        for state in &system.states {
            state.write(&device, particles, 0);
        }
        system.trails.reset();
        system.display.reset();
        system
            .set_flash_recovery(if regrab { 0.1 } else { 0.65 })
            .unwrap();
        system.set_dragging(true);
        for _ in 0..120 {
            step(&device, &mut system, 0.0125);
        }
        if retract {
            system.set_dragging(false);
            for _ in 0..26 {
                step(&device, &mut system, 0.0125);
            }
        }
        if regrab {
            system.set_dragging(false);
            for _ in 0..4 {
                step(&device, &mut system, 0.0125);
            }
            system.set_dragging(true);
            for _ in 0..2 {
                step(&device, &mut system, 0.0125);
            }
            assert!(system.particles_visible());
            assert!(system.display.flash_boost() < 1.);
        }
        render(&mut system, 1.)
    };
    for (retract, regrab) in [(false, false), (true, false), (false, true)] {
        let near_first = overlap(&[p, far], retract, regrab);
        let far_first = overlap(&[far, p], retract, regrab);
        assert!(red_sum(&near_first) > 0);
        assert!(
            near_first == far_first,
            "flashing trails must not depend on overlap draw order"
        );
    }
}

fn device() -> Device {
    pollster::block_on(Device::create_headless_with_options(
        belfast::DeviceOptions {
            format: wgpu::TextureFormat::Rgba8Unorm,
            ..Default::default()
        },
    ))
    .unwrap()
}

fn particle(x: f32, phase: f32) -> Particle {
    Particle {
        position_phase: [x, 1., -3., phase],
        velocity_frequency: [0., 0., 0., 0.22],
        flash: [
            DEFAULT_FLASH_DURATION_SECONDS,
            0.,
            DEFAULT_FLASH_DURATION_SECONDS,
            0.,
        ],
    }
}

fn camera() -> SortCamera {
    SortCamera::new(
        Mat4::IDENTITY,
        Mat4::perspective_rh(1., 1., 0.02, 100.),
        [64, 64],
    )
}

fn read_buffer(device: &Device, source: &wgpu::Buffer, size: u64) -> Vec<u8> {
    let readback = device.gpu().create_buffer(&wgpu::BufferDescriptor {
        label: Some("FireflyTestReadback"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.gpu().create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(source, 0, &readback, 0, size);
    device.queue().submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .gpu()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    rx.recv().unwrap().unwrap();
    let bytes = readback.slice(..).get_mapped_range().unwrap().to_vec();
    readback.unmap();
    bytes
}

fn step(device: &Device, system: &mut FireflySystem, dt: f32) -> Vec<Particle> {
    system.update(device, dt, &camera());
    let mut encoder = device.gpu().create_command_encoder(&Default::default());
    system.simulate(&mut encoder);
    device.queue().submit([encoder.finish()]);
    let bytes = read_buffer(
        device,
        system.states[system.current].gpu(),
        u64::from(system.count) * std::mem::size_of::<Particle>() as u64,
    );
    bytemuck::cast_slice(&bytes).to_vec()
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_phases_follow_shortest_arc_and_zero_dt_preserves_state() {
    let device = device();
    let settings = Settings {
        noise_strength: 0.,
        phase_jitter: 0.,
        ..Default::default()
    };
    let input = [particle(-0.1, 0.98), particle(0.1, 0.02)];
    let mut system = FireflySystem::with_particles(&device, &input, settings);
    assert_eq!(step(&device, &mut system, 0.), input);
    let output = step(&device, &mut system, 0.05);
    assert!(
        output[0].position_phase[3] > 0.98 + 0.22 * 0.05,
        "lagging across zero must speed up"
    );
    assert!(
        output[1].position_phase[3] < 0.02 + 0.22 * 0.05,
        "leading across zero must slow down"
    );
    for _ in 0..10 {
        step(&device, &mut system, 0.05);
    }
    let output = step(&device, &mut system, 0.05);
    assert!(output
        .iter()
        .all(|p| (0.0..1.0).contains(&p.position_phase[3])));
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_separation_cohesion_alignment_and_isolated_motion() {
    let device = device();
    let clean = Settings {
        noise_strength: 0.,
        phase_jitter: 0.,
        drag: 0.,
        ..Default::default()
    };
    let mut near =
        FireflySystem::with_particles(&device, &[particle(0.77, 0.2), particle(0.83, 0.2)], clean);
    let p = step(&device, &mut near, 0.05);
    assert!(
        p[0].velocity_frequency[0] < 0. && p[1].velocity_frequency[0] > 0.,
        "near neighbors separate across grid boundary"
    );
    let mut far =
        FireflySystem::with_particles(&device, &[particle(-0.35, 0.2), particle(0.35, 0.2)], clean);
    let p = step(&device, &mut far, 0.05);
    assert!(
        p[0].velocity_frequency[0] > 0. && p[1].velocity_frequency[0] < 0.,
        "far neighbors cohere"
    );
    let mut moving = particle(0.3, 0.2);
    moving.velocity_frequency[2] = 0.2;
    let mut aligned = FireflySystem::with_particles(&device, &[particle(-0.3, 0.2), moving], clean);
    assert!(
        step(&device, &mut aligned, 0.05)[0].velocity_frequency[2] > 0.,
        "follow neighbor direction"
    );
    let mut lone = particle(0., 0.2);
    lone.velocity_frequency[0] = 0.1;
    let mut isolated = FireflySystem::with_particles(&device, &[lone], clean);
    let p = step(&device, &mut isolated, 0.05)[0];
    assert!((p.position_phase[0] - 0.005).abs() < 1e-5);
    assert!((p.position_phase[3] - 0.211).abs() < 1e-5);
    let mut coincident = FireflySystem::with_particles(&device, &[particle(0., 0.2); 2], clean);
    let p = step(&device, &mut coincident, 0.05);
    assert!(
        Vec3::from_slice(&p[0].position_phase).distance(Vec3::from_slice(&p[1].position_phase))
            > 0.
    );
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_dense_cell_visits_every_neighbor_without_bucket_overflow() {
    let device = device();
    // 129 co-located particles exceed common 64/128-entry bucket capacities.
    // The two distinct phases make a dropped neighbor observable in every rate.
    let mut input = vec![particle(0., 0.3); 129];
    input[0].position_phase[3] = 0.1;
    input[128].position_phase[3] = 0.45;
    let settings = Settings {
        phase_jitter: 0.,
        phase_coupling: 0.65,
        ..Default::default()
    };
    let mut system = FireflySystem::with_particles(&device, &input, settings);
    let output = step(&device, &mut system, 0.05);
    // Hand-derived phase deltas: 25.75/128, -0.05/128, and -19.4/128.
    assert!((output[0].position_phase[3] - 0.117_538_09).abs() < 2e-7);
    for p in &output[1..128] {
        assert!((p.position_phase[3] - 0.310_987_3).abs() < 2e-7);
    }
    assert!((output[128].position_phase[3] - 0.456_074_2).abs() < 2e-7);
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_full_population_remains_bounded_and_moves() {
    let device = device();
    let settings = Settings::default();
    let mut input = initial_particles(&settings, PARTICLE_COUNT);
    input[0].position_phase[1] = settings.min[1] + BODY_RADIUS;
    input[0].velocity_frequency[1] = -4.;
    input[1].position_phase[1] = settings.max[1] - BODY_RADIUS;
    input[1].velocity_frequency[1] = 4.;
    let mut system = FireflySystem::with_particles(&device, &input, settings);
    let mut output = Vec::new();
    for _ in 0..30 {
        output = step(&device, &mut system, 0.1);
    }
    assert_eq!(output.len(), PARTICLE_COUNT as usize);
    for p in &output {
        assert!(bytemuck::cast_slice::<_, f32>(std::slice::from_ref(p))
            .iter()
            .all(|v| v.is_finite()));
        for axis in 0..3 {
            assert!(p.position_phase[axis] >= settings.min[axis] + BODY_RADIUS - 1e-5);
            assert!(p.position_phase[axis] <= settings.max[axis] - BODY_RADIUS + 1e-5);
        }
        assert!(Vec3::from_slice(&p.velocity_frequency).length() <= settings.max_speed + 1e-5);
    }
    assert!(
        output
            .iter()
            .zip(input)
            .filter(|(a, b)| Vec3::from_slice(&a.position_phase)
                .distance(Vec3::from_slice(&b.position_phase))
                > 0.01)
            .count()
            > PARTICLE_COUNT as usize * 9 / 10
    );
}

#[test]
#[ignore = "requires a GPU"]
fn gpu_glow_pulses_and_forest_depth_occludes_only_distant_fireflies() {
    use crate::{ply::GaussianCloud, splat::SplatRenderer};
    let device = device();
    let mut camera = camera();
    // Resolve the small light across several pixels in this 64px test target.
    camera.projection = Mat4::perspective_rh(0.4, 1., 0.02, 100.);
    let mut cloud = GaussianCloud {
        source_count: 1,
        positions: vec![[0., 0., -2., 0.25]],
        // Isotropic covariance; opaque forest surface centered in front of the light.
        attributes: vec![[0x00003c00, 0x3c000000, 0x3c000000, 0xffffffff]],
        sort_positions: vec![[0., 0., -2., 1.5]],
    };
    let mut forest = SplatRenderer::new(&device, &mut cloud).unwrap();
    forest.set_indices(&device, &[0]).unwrap();
    forest.update_camera(&device, &camera);
    let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
        label: Some("FireflyGlowTest"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: device.format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target = texture.create_view(&Default::default());
    let depth = depth_view(&device, [64, 64]);
    let staging = device.gpu().create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 64 * 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let render_after_release = |brightness: f32,
                                z: f32,
                                occluded: bool,
                                drag_seconds: f32,
                                recovery: f32,
                                release_seconds: f32,
                                glow: bool| {
        let p = Particle {
            position_phase: [0., 0., z, 0.],
            velocity_frequency: [0., 0., 0., 0.22],
            flash: [0.05, brightness, DEFAULT_FLASH_DURATION_SECONDS, 0.],
        };
        let mut system = FireflySystem::with_particles(&device, &[p], Settings::default());
        system.set_glow(glow);
        system.set_flash_recovery(recovery).unwrap();
        system.set_render_mode(RenderMode::Drag);
        system.set_dragging(true);
        system.update(&device, 0., &camera);
        let mut remaining = drag_seconds;
        while remaining > 0. {
            let dt = remaining.min(0.1);
            system.update(&device, dt, &camera);
            remaining -= dt;
        }
        if release_seconds > 0. {
            system.set_dragging(false);
            let mut remaining = release_seconds;
            while remaining > 0. {
                let dt = remaining.min(0.1);
                system.update(&device, dt, &camera);
                remaining -= dt;
            }
        }
        let mut encoder = device.gpu().create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    stencil_ops: None,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if occluded {
                forest.draw_depth(&mut pass);
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    stencil_ops: None,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            system.draw(&mut pass);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(64),
                },
            },
            wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
        );
        device.queue().submit([encoder.finish()]);
        read_buffer(&device, &staging, 64 * 256)
            .chunks_exact(4)
            .map(|p| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]))
            .sum::<u32>()
    };
    let render = |brightness, z, occluded, drag_seconds| {
        render_after_release(brightness, z, occluded, drag_seconds, 0.65, 0., true)
    };
    assert_eq!(
        render_after_release(0., -3., false, 0.25, 0.65, 1., true),
        0
    );
    assert!(
        render_after_release(0., -3., false, 0.25, 2., 1., true) > 0,
        "long recovery fades particle brightness and opacity together"
    );
    assert_eq!(render_after_release(0., -3., false, 0.25, 2., 2.1, true), 0);
    let bright = render(1., -3., false, 0.);
    // Resolve the narrow core above a pixel in this 64px target.
    let near_bright = render(1., -1., false, 0.);
    let core_only = render_after_release(1., -1., false, 0., 0.65, 0., false);
    assert!(
        core_only > 0 && core_only < near_bright,
        "disabling glow removes the halo while retaining a visible core: {core_only} / {near_bright}"
    );
    assert_eq!(render_after_release(0., -3., false, 0., 0.65, 0., false), 0);
    assert_eq!(render_after_release(1., -3., true, 0., 0.65, 0., false), 0);
    let dark = render(0., -3., false, 0.);
    let fading = render(1., -3., false, 0.125);
    let waking = render(0., -3., false, 0.125);
    assert!(
        waking > 0 && waking < fading,
        "dark particles gradually light during drag"
    );
    assert!(
        fading > 0 && fading < bright,
        "particle emission fades during drag"
    );
    assert_eq!(
        render(1., -3., false, 0.25),
        0,
        "particle emission disappears at full trail visibility"
    );
    assert!(
        bright > 20 && bright > dark * 10,
        "cycle must have a visible pulse: {bright} / {dark}"
    );
    assert_eq!(
        render(1., -3., true, 0.),
        0,
        "forest must occlude a light behind it"
    );
    assert!(
        render(1., -1., true, 0.) > bright,
        "light in front of forest stays visible"
    );
}
