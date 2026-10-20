use super::*;

#[test]
fn compute_and_instanced_material_shaders_validate() {
    for source in [
        format!("{STATE_SHADER}\n{NOISE_SHADER}\n{UPDATE_SHADER}"),
        format!(
            "{}\n{STATE_SHADER}\n{TRANSFORM_SHADER}\n{DRAW_SHADER}",
            crate::model::shader_source()
        ),
        format!(
            "{}\n{STATE_SHADER}\n{TRANSFORM_SHADER}\n{SHADOW_SHADER}",
            crate::shadow::DEPTH_SHADER
        ),
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

fn map_buffer(device: &Device, buffer: &wgpu::Buffer) -> Vec<u8> {
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    device
        .gpu()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    rx.recv().unwrap().unwrap();
    let bytes = buffer.slice(..).get_mapped_range().unwrap().to_vec();
    buffer.unmap();
    bytes
}

#[test]
#[ignore = "requires a GPU; verifies petal/branch casting, receiving, scale and ping-pong shadows"]
fn gpu_petals_cast_and_receive_pcf_shadows() {
    use crate::{
        model_data::{ImageData, PrimitiveData, Vertex},
        shadow::{fit_shadow_camera, ShadowMap},
    };
    let device = pollster::block_on(Device::create_headless_with_options(
        belfast::DeviceOptions {
            format: wgpu::TextureFormat::Rgba8Unorm,
            ..Default::default()
        },
    ))
    .unwrap();
    let quad = |half: [f32; 2], center: Vec3| ModelData {
        primitives: vec![PrimitiveData {
            vertices: [[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]]
                .map(|p| Vertex {
                    position: (center + Vec3::new(p[0] * half[0], p[1] * half[1], 0.)).to_array(),
                    normal: [0., 0., 1.],
                    uv: [0., 0.],
                })
                .to_vec(),
            indices: vec![0, 1, 2, 0, 2, 3],
            base_color: ImageData {
                width: 1,
                height: 1,
                rgba: vec![180, 180, 180, 255],
            },
            normal_map: ImageData {
                width: 1,
                height: 1,
                rgba: vec![128, 128, 255, 255],
            },
            base_factor: [1.; 4],
            normal_scale: 0.,
            double_sided: true,
        }],
    };
    let light = PointLight {
        position: [0., 4., 6.],
        color: [1.; 3],
        intensity: 40.,
    };
    let light_vp = fit_shadow_camera(
        Vec3::from(light.position),
        Vec3::new(-2., -2., -0.1),
        Vec3::new(2., 2., 2.5),
    )
    .unwrap();
    let shadow = ShadowMap::new(&device, 512, light_vp, 0.85).unwrap();
    let camera = Mat4::orthographic_rh(-2., 2., -2., 2., 0.1, 20.)
        * Mat4::look_at_rh(Vec3::new(0., 0., 6.), Vec3::ZERO, Vec3::Y);
    let receiver =
        ModelRenderer::new_shadowed(&device, &quad([2., 2.], Vec3::ZERO), &shadow).unwrap();
    receiver.update_camera(&device, &camera.to_cols_array());
    receiver.update_light(&device, light);
    let branch =
        ModelRenderer::new_shadowed(&device, &quad([0.6, 0.6], Vec3::new(0., 0.8, 2.)), &shadow)
            .unwrap();
    let mut particles = initial_particles(&[Vec3::ZERO], Mat4::IDENTITY, 1).unwrap();
    particles[0].position_life = [0., 0., 1., 5.];
    particles[0].origin_lifetime = [0., 0., 1., 10.];
    particles[0].velocity_angle = [0., 0., 0., 0.];
    particles[0].axis_speed = [0., 0., 1., 0.];
    particles[0].appearance = [1., 0., 1., 1.];
    let mut petals =
        PetalSystem::new_shadowed(&device, &quad([0.3, 0.45], Vec3::ZERO), &particles, &shadow)
            .unwrap();
    petals.set_scale(&device, 1.).unwrap();
    petals.update(&device, 0., &camera.to_cols_array());
    petals.update_light(&device, light);
    let extent = wgpu::Extent3d {
        width: 128,
        height: 128,
        depth_or_array_layers: 1,
    };
    let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
        label: Some("ShadowTestColor"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: device.format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target = texture.create_view(&Default::default());
    let depth = crate::model::depth_view(&device, [128, 128]);
    let readback = device.gpu().create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 128 * 512,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let render =
        |petals: &PetalSystem, cast_branch: bool, cast_petals: bool, receive_petals: bool| {
            let mut encoder = device.gpu().create_command_encoder(&Default::default());
            {
                let mut pass = shadow
                    .map
                    .begin_depth_pass(&mut encoder, wgpu::LoadOp::Clear(1.));
                if cast_branch {
                    branch.draw_shadow(&mut pass);
                }
                if cast_petals {
                    petals.draw_shadow(&mut pass);
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
                            load: wgpu::LoadOp::Clear(1.),
                            store: wgpu::StoreOp::Discard,
                        }),
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                if receive_petals {
                    petals.draw(&mut pass);
                } else {
                    receiver.draw(&mut pass);
                }
            }
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(512),
                        rows_per_image: Some(128),
                    },
                },
                extent,
            );
            device.queue().submit([encoder.finish()]);
            map_buffer(&device, &readback)
        };
    let darkened = |lit: &[u8], shaded: &[u8]| {
        lit.chunks_exact(4)
            .zip(shaded.chunks_exact(4))
            .filter(|(a, b)| u16::from(a[0]) > u16::from(b[0]) + 12)
            .count()
    };
    let lit = render(&petals, false, false, false);
    let cast = render(&petals, false, true, false);
    let full = darkened(&lit, &cast);
    assert!(full > 200, "Petal must cast a visible shadow: {full}");
    assert!(
        darkened(&lit, &render(&petals, true, false, false)) > 200,
        "Branches must cast shadows"
    );
    let petal_lit = render(&petals, false, false, true);
    let petal_shaded = render(&petals, true, false, true);
    assert!(
        darkened(&petal_lit, &petal_shaded) > 150,
        "Petals must receive branch shadows"
    );
    petals.set_scale(&device, 0.5).unwrap();
    let half = darkened(&lit, &render(&petals, false, true, false));
    assert!(
        half > full / 8 && half < full / 2,
        "Scale must resize the shadow: {full} -> {half}"
    );
    petals.set_scale(&device, 1.).unwrap();
    // After compute swaps the ping-pong index, shadow drawing must use that state.
    let mut moved = particles.clone();
    moved[0].position_life[0] = 0.8;
    petals.buffers.states[petals.current].write(&device, &moved, 0);
    let expected = render(&petals, false, true, false);
    petals.update(&device, 0., &camera.to_cols_array());
    let mut encoder = device.gpu().create_command_encoder(&Default::default());
    petals.simulate(&mut encoder);
    device.queue().submit([encoder.finish()]);
    assert_eq!(render(&petals, false, true, false), expected);
    assert_ne!(expected, cast, "Moving a petal must move its shadow");
    moved[0].appearance[2] = 0.;
    petals.buffers.states[petals.current].write(&device, &moved, 0);
    assert_eq!(
        render(&petals, false, true, false),
        lit,
        "Zero lifetime scale must not cast a ghost shadow"
    );
}

fn step_and_read(device: &Device, system: &mut PetalSystem, dt: f32) -> Vec<Particle> {
    system.update(device, dt, &Mat4::IDENTITY.to_cols_array());
    dispatch_and_read(device, system)
}

fn dispatch_and_read(device: &Device, system: &mut PetalSystem) -> Vec<Particle> {
    let mut encoder = device.gpu().create_command_encoder(&Default::default());
    system.simulate(&mut encoder);
    let state = &system.buffers.states[system.current];
    let readback = device.gpu().create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: state.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(state.gpu(), 0, &readback, 0, state.size());
    device.queue().submit([encoder.finish()]);
    let bytes = map_buffer(device, &readback);
    bytes
        .chunks_exact(80)
        .map(bytemuck::pod_read_unaligned)
        .collect()
}

#[test]
#[ignore = "requires a GPU; samples full petal lifetimes against the compact shadow envelope"]
fn gpu_petal_flights_fit_shadow_envelope() {
    use crate::scene::{MAGNOLIA_BASE, MAGNOLIA_HEIGHT};
    let asset = |name: &str| {
        std::fs::read(format!("{}/public/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    };
    let branches = ModelData::parse(&asset("magnolia-branches.glb")).unwrap();
    let transform = branches
        .placement_transform(MAGNOLIA_BASE, MAGNOLIA_HEIGHT)
        .unwrap();
    let model = ModelData::parse(&asset("magnolia-petal.glb")).unwrap();
    let emitters = parse_emitters(&asset("magnolia-emitter-positions.json")).unwrap();
    let (min, max) = motion_bounds(&emitters, transform, &model);
    let mesh_radius = model
        .primitives
        .iter()
        .flat_map(|p| &p.vertices)
        .map(|v| Vec3::from(v.position).length())
        .fold(0f32, f32::max);
    let device = pollster::block_on(Device::create_headless()).unwrap();
    let mut particles = initial_particles(&emitters, transform, 10_000).unwrap();
    // Exercise every emitter with the longest life, fastest drift and largest mesh.
    for p in &mut particles {
        p.position_life[3] = LIFETIME_RANGE[1];
        p.origin_lifetime[3] = LIFETIME_RANGE[1];
        p.appearance[0] = transform.x_axis.truncate().length() * SIZE_MULTIPLIER_RANGE[1];
        p.appearance[3] = SPEED_MULTIPLIER_RANGE[1];
    }
    let mut system = PetalSystem::new(&device, &model, &particles).unwrap();
    let mut observed_min = Vec3::splat(f32::INFINITY);
    let mut observed_max = Vec3::splat(f32::NEG_INFINITY);
    for start_time in [0., 37., 137., 1000., 4000.] {
        system.reset_particles(&device, &particles).unwrap();
        system.simulation_time = start_time;
        for _ in 0..120 {
            for p in step_and_read(&device, &mut system, 0.1) {
                let position = Vec3::from_slice(&p.position_life);
                let radius = Vec3::splat(mesh_radius * p.appearance[0] * p.appearance[2]);
                observed_min = observed_min.min(position - radius);
                observed_max = observed_max.max(position + radius);
                assert!((position - radius).cmpge(min).all() && (position + radius).cmple(max).all(),
                    "Petal at {position:?} with radius {radius:?} escaped {min:?}..{max:?}, time {}", system.simulation_time);
            }
        }
    }
    eprintln!(
        "Observed flight {observed_min:?}..{observed_max:?}; shadow envelope {min:?}..{max:?}"
    );
}

#[test]
#[ignore = "requires a GPU; verifies velocity += force, position += velocity, then friction"]
fn gpu_force_integration_applies_friction_after_position() {
    let device = pollster::block_on(Device::create_headless()).unwrap();
    let mut p = initial_particles(&[Vec3::ZERO], Mat4::IDENTITY, 1).unwrap();
    p[0].position_life = [0., 0., 0., 5.];
    p[0].origin_lifetime = [0., 0., 0., 10.];
    p[0].velocity_angle = [0.2, 0.3, 0.4, 0.];
    p[0].appearance[3] = 1.;
    let mut system = PetalSystem::new(&device, &ModelData { primitives: vec![] }, &p).unwrap();
    system.update(&device, 1. / 120., &Mat4::IDENTITY.to_cols_array());
    system.params.write(
        &device,
        &[SimParams {
            wind_friction: [1., 2., -0.5, 0.81],
            noise: [1., 0., 0., 0.],
            lifecycle: [0.2, 0.1, 0.2, 0.],
            delta_seconds: 1. / 120.,
            simulation_time: 0.,
            count: 1,
            padding: 0,
        }],
        0,
    );
    let out = dispatch_and_read(&device, &mut system);
    for (got, want) in out[0].position_life[..3]
        .iter()
        .zip([0.001736111, 0.002638889, 0.003298611])
    {
        assert!((got - want).abs() < 1e-7);
    }
    for (got, want) in out[0].velocity_angle[..3]
        .iter()
        .zip([0.1875, 0.285, 0.35625])
    {
        assert!((got - want).abs() < 1e-6);
    }
}

#[test]
#[ignore = "requires a GPU; speed multipliers change displacement without accelerating lifetime"]
fn gpu_per_petal_speed_scales_motion_without_compounding_velocity() {
    let device = pollster::block_on(Device::create_headless()).unwrap();
    let mut particles = initial_particles(&[Vec3::ZERO], Mat4::IDENTITY, 2).unwrap();
    for (p, speed) in particles.iter_mut().zip([0.5, 2.]) {
        p.position_life = [0., 0., 0., 5.];
        p.origin_lifetime = [0., 0., 0., 10.];
        p.velocity_angle = [0.6, 0.3, 0., 0.];
        p.appearance[3] = speed;
    }
    let mut system =
        PetalSystem::new(&device, &ModelData { primitives: vec![] }, &particles).unwrap();
    system.update(&device, 1. / 120., &Mat4::IDENTITY.to_cols_array());
    system.params.write(
        &device,
        &[SimParams {
            wind_friction: [0., 0., 0., 1.],
            noise: [1., 0., 0., 0.],
            lifecycle: [0.2, 0.1, 0.2, 0.],
            delta_seconds: 1. / 120.,
            simulation_time: 0.,
            count: 2,
            padding: 0,
        }],
        0,
    );
    let first = dispatch_and_read(&device, &mut system);
    assert!((first[0].position_life[0] - 0.0025).abs() < 1e-7);
    assert!((first[1].position_life[0] - 0.0100).abs() < 1e-7);
    assert_eq!(first[0].velocity_angle[..3], first[1].velocity_angle[..3]);
    assert_eq!(first[0].position_life[3], first[1].position_life[3]);
    let second = dispatch_and_read(&device, &mut system);
    assert!((second[0].position_life[0] - 0.005).abs() < 1e-7);
    assert!((second[1].position_life[0] - 0.020).abs() < 1e-7);
}

#[test]
#[ignore = "requires a GPU; randomized initial phases move immediately without losing the held stage"]
fn gpu_initial_phases_mix_held_and_moving_petals_on_first_frame() {
    let device = pollster::block_on(Device::create_headless()).unwrap();
    let particles = initial_particles(&[Vec3::ZERO], Mat4::IDENTITY, PARTICLE_COUNT).unwrap();
    let mut system =
        PetalSystem::new(&device, &ModelData { primitives: vec![] }, &particles).unwrap();
    let out = step_and_read(&device, &mut system, 1. / 60.);
    let moved = particles
        .iter()
        .zip(&out)
        .filter(|(before, after)| {
            (Vec3::from_slice(&before.position_life) - Vec3::from_slice(&after.position_life))
                .length()
                > 1e-7
        })
        .count();
    assert!(
        (7000..8500).contains(&moved),
        "Most petals should already move on frame one: {moved}"
    );
    for p in out
        .iter()
        .filter(|p| p.position_life[3] / p.origin_lifetime[3] > 0.8)
    {
        assert_eq!(p.position_life[..3], p.origin_lifetime[..3]);
        assert_eq!(p.velocity_angle[..3], [0.; 3]);
    }
}

#[test]
#[ignore = "requires a GPU; checks 10k/100k/500k resizing and both ping-pong buffers"]
fn gpu_petal_count_switches_up_and_down_without_stale_bindings() {
    let device = pollster::block_on(Device::create_headless()).unwrap();
    let initial = initial_particles(&[Vec3::ZERO], Mat4::IDENTITY, 10_000).unwrap();
    let mut system =
        PetalSystem::new(&device, &ModelData { primitives: vec![] }, &initial).unwrap();
    // Change count after an odd frame so the old read/write index is not zero.
    step_and_read(&device, &mut system, 0.01);
    for count in [100_000, 500_000, 10_000] {
        let mut particles = initial_particles(&[Vec3::ZERO], Mat4::IDENTITY, count).unwrap();
        for p in &mut particles {
            p.position_life[3] = 2.;
        }
        system.reset_particles(&device, &particles).unwrap();
        assert_eq!(system.count(), count);
        let first = step_and_read(&device, &mut system, 0.01);
        assert_eq!(first.len(), count as usize);
        assert!((first.last().unwrap().position_life[3] - 1.99).abs() < 1e-5);
        let second = step_and_read(&device, &mut system, 0.01);
        assert!((second.last().unwrap().position_life[3] - 1.98).abs() < 1e-5);
        assert!(system.reset_particles(&device, &[]).is_err());
        assert_eq!(
            system.count(),
            count,
            "A rejected resize must retain the current particles"
        );
    }
}

#[test]
#[ignore = "requires a GPU; checks scale-in/out at birth, midlife and death"]
fn gpu_lifetime_scale_envelope_and_respawn() {
    let device = pollster::block_on(Device::create_headless()).unwrap();
    let mut particles = initial_particles(&[Vec3::ZERO], Mat4::IDENTITY, 6).unwrap();
    for (p, remaining) in particles.iter_mut().zip([10., 9.5, 9., 5., 1., 0.]) {
        p.position_life[3] = remaining + 0.001;
        p.origin_lifetime[3] = 10.;
    }
    let mut system =
        PetalSystem::new(&device, &ModelData { primitives: vec![] }, &particles).unwrap();
    let out = step_and_read(&device, &mut system, 0.001);
    for (p, expected) in out.iter().zip([0., 0.5, 1., 1., 0.5, 0.]) {
        assert!(
            (p.appearance[2] - expected).abs() < 1e-4,
            "Scale was {} not {expected}",
            p.appearance[2]
        );
    }
    assert_eq!(out[5].position_life, particles[5].origin_lifetime);
    assert_eq!(out[5].appearance[0], particles[5].appearance[0]);
    assert_eq!(out[5].appearance[3], particles[5].appearance[3]);
    assert_eq!(
        out[5].velocity_angle,
        [0., 0., 0., particles[5].appearance[1]]
    );
}

#[test]
#[ignore = "requires a GPU; verifies a smooth, nonzero, divergence-free curl field"]
fn gpu_curl_field_is_spatially_varying_and_divergence_free() {
    let device = pollster::block_on(Device::create_headless()).unwrap();
    let source = [
        NOISE_SHADER,
        r#"
        @group(0) @binding(0) var<storage, read_write> samples: array<vec4<f32>>;
        @compute @workgroup_size(64) fn probe(@builtin(global_invocation_id) id: vec3<u32>) {
            if id.x >= 9u { return; }
            let offsets = array<vec3<f32>, 9>(vec3(0.),
                vec3(0.001, 0., 0.), vec3(-0.001, 0., 0.),
                vec3(0., 0.001, 0.), vec3(0., -0.001, 0.),
                vec3(0., 0., 0.001), vec3(0., 0., -0.001),
                vec3(1.1, 0.7, -0.8), vec3(0.17, 0.11, 0.13));
            samples[id.x] = vec4(curl_noise(vec3(-0.37, 1.42, -2.81) + offsets[id.x]), 0.);
        }
    "#,
    ]
    .join("\n");
    let compute = Compute::new(
        &device,
        &source,
        ComputeOptions {
            entry_point: "probe",
            ..ComputeOptions::new("CurlProbe")
        },
    );
    let output = Buffer::create(
        &device,
        9 * 16,
        BufferUsage::storage() | wgpu::BufferUsages::COPY_SRC,
        "CurlOutput",
    );
    let binding = BindGroup::create(
        &device,
        &compute.get_bind_group_layout(0),
        &[wgpu::BindGroupEntry {
            binding: 0,
            resource: output.gpu().as_entire_binding(),
        }],
        "CurlProbeBinding",
    );
    let readback = Buffer::create(
        &device,
        9 * 16,
        wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        "CurlReadback",
    );
    let mut encoder = device.gpu().create_command_encoder(&Default::default());
    compute.run(&mut encoder, Some(&binding), [1, 1, 1], None);
    encoder.copy_buffer_to_buffer(output.gpu(), 0, readback.gpu(), 0, 9 * 16);
    device.queue().submit([encoder.finish()]);
    let bytes = map_buffer(&device, readback.gpu());
    let samples: Vec<[f32; 4]> = bytes
        .chunks_exact(16)
        .map(bytemuck::pod_read_unaligned)
        .collect();
    assert!(samples.iter().flatten().all(|v| v.is_finite()));
    let v = |i: usize| Vec3::from_slice(&samples[i]);
    assert!(v(0).length() > 0.01);
    assert!(
        (v(0) - v(7)).length() > 0.1,
        "Different positions must feel different forces"
    );
    assert!(
        (v(0) - v(8)).length() > 0.01,
        "Moving the field over time must change the force"
    );
    assert!(
        (v(1) - v(2)).length() < 0.1,
        "Nearby positions must see a continuous field"
    );
    let divergence = ((samples[1][0] - samples[2][0])
        + (samples[3][1] - samples[4][1])
        + (samples[5][2] - samples[6][2]))
        / 0.002;
    assert!(divergence.abs() < 0.01, "curl divergence = {divergence}");
}

#[test]
#[ignore = "requires a GPU; holds new petals still and scales them in before release"]
fn gpu_new_petals_hold_position_and_rotation_for_first_twenty_percent() {
    let device = pollster::block_on(Device::create_headless()).unwrap();
    let mesh = ModelData { primitives: vec![] };
    let mut particles = initial_particles(&[Vec3::new(1., 2., -6.)], Mat4::IDENTITY, 1).unwrap();
    particles[0].position_life[3] = 10.;
    particles[0].origin_lifetime[3] = 10.;
    let mut system = PetalSystem::new(&device, &mesh, &particles).unwrap();
    let first = step_and_read(&device, &mut system, 0.1);
    assert_eq!(first[0].position_life[..3], particles[0].position_life[..3]);
    assert_eq!(first[0].velocity_angle, particles[0].velocity_angle);
    assert!((first[0].appearance[2] - 0.028).abs() < 1e-4);
    for _ in 0..18 {
        step_and_read(&device, &mut system, 0.1);
    }
    let last_held = step_and_read(&device, &mut system, 0.05);
    assert_eq!(
        last_held[0].position_life[..3],
        particles[0].position_life[..3]
    );
    assert_eq!(last_held[0].velocity_angle, particles[0].velocity_angle);
    assert!((last_held[0].appearance[2] - 1.).abs() < 1e-6);
    let released = step_and_read(&device, &mut system, 0.1);
    assert!(released[0].position_life[1] > last_held[0].position_life[1]);
    assert_ne!(
        released[0].velocity_angle[3],
        last_held[0].velocity_angle[3]
    );
}

#[test]
#[ignore = "requires a GPU; verifies compute ping-pong, lifetime, rotation and 10,000-particle dispatch"]
fn gpu_petals_move_rotate_respawn_and_are_frame_rate_independent() {
    let device = pollster::block_on(Device::create_headless()).unwrap();
    // No meshes needed to observe the real compute output.
    let mesh = ModelData { primitives: vec![] };
    let mut particles =
        initial_particles(&[Vec3::new(1., 2., -6.)], Mat4::IDENTITY, PARTICLE_COUNT).unwrap();
    for p in &mut particles {
        p.position_life[3] = 2.;
    }
    particles[0].position_life[3] = 0.001;
    particles[0].position_life[0] += 9.;
    particles[0].velocity_angle = [3., 4., 5., 1.];
    let mut system = PetalSystem::new(&device, &mesh, &particles).unwrap();
    let first = step_and_read(&device, &mut system, 0.1);
    assert_eq!(first[0].position_life, particles[0].origin_lifetime);
    assert_eq!(
        first[0].velocity_angle,
        [0., 0., 0., particles[0].appearance[1]]
    );
    // Check all invocations, including the incomplete final 64-wide workgroup.
    for (before, after) in particles.iter().zip(&first).skip(1) {
        assert!((after.position_life[3] - 1.9).abs() < 1e-5);
        assert!(
            (Vec3::from_slice(&after.position_life) - Vec3::from_slice(&before.position_life))
                .length()
                > 0.00001
        );
        assert_eq!(after.origin_lifetime, before.origin_lifetime);
        assert_eq!(after.axis_speed, before.axis_speed);
        let delta = after.velocity_angle[3] - before.velocity_angle[3];
        assert!((delta.sin() - (before.axis_speed[3] * 0.1).sin()).abs() < 1e-5);
    }
    let second = step_and_read(&device, &mut system, 0.1);
    assert!(
        second[1].position_life[1] > first[1].position_life[1],
        "Second frame must consume the first frame's output"
    );
    assert!((second[1].position_life[3] - 1.8).abs() < 1e-5);

    let mut half_step = PetalSystem::new(&device, &mesh, &particles).unwrap();
    step_and_read(&device, &mut half_step, 0.05);
    let twice = step_and_read(&device, &mut half_step, 0.05);
    for index in [1, 127, 9999] {
        for component in 0..4 {
            assert!(
                (twice[index].position_life[component] - first[index].position_life[component])
                    .abs()
                    < 1e-5
            );
        }
    }
    let paused = step_and_read(&device, &mut system, 0.);
    for (a, b) in second.iter().zip(paused) {
        assert_eq!(a.position_life, b.position_life);
    }
}

#[test]
#[ignore = "requires a GPU; verifies actual GLB instancing and independent axis-angle rotation"]
fn gpu_draws_two_petals_at_distinct_positions_and_rotations() {
    let device = pollster::block_on(Device::create_headless_with_options(
        belfast::DeviceOptions {
            format: wgpu::TextureFormat::Rgba8Unorm,
            ..Default::default()
        },
    ))
    .unwrap();
    let data = ModelData::parse(
        &std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/public/magnolia-petal.glb"
        ))
        .unwrap(),
    )
    .unwrap();
    let mut particles = initial_particles(
        &[Vec3::new(-0.5, 0., 0.5), Vec3::new(0.5, 0., 0.5)],
        Mat4::IDENTITY,
        2,
    )
    .unwrap();
    for (i, p) in particles.iter_mut().enumerate() {
        p.position_life = [if i == 0 { -0.5 } else { 0.5 }, 0., 0.5, 5.];
        p.axis_speed = [0., 0., 1., 1.];
        p.velocity_angle[3] = if i == 0 {
            0.
        } else {
            std::f32::consts::FRAC_PI_2
        };
        p.appearance[0] = 3.;
        p.appearance[2] = 1.;
    }
    let mut petals = PetalSystem::new(&device, &data, &particles).unwrap();
    petals.set_scale(&device, 1.).unwrap();
    petals.update(&device, 0., &Mat4::IDENTITY.to_cols_array());
    let size = wgpu::Extent3d {
        width: 128,
        height: 128,
        depth_or_array_layers: 1,
    };
    let texture = device.gpu().create_texture(&wgpu::TextureDescriptor {
        label: None,
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: device.format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let depth = crate::model::depth_view(&device, [128, 128]);
    let readback = device.gpu().create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 128 * 512,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let render = |petals: &PetalSystem| {
        let mut encoder = device.gpu().create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Discard,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            petals.draw(&mut pass);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(512),
                    rows_per_image: Some(128),
                },
            },
            size,
        );
        device.queue().submit([encoder.finish()]);
        map_buffer(&device, &readback)
    };
    let bytes = render(&petals);
    let mut bounds = [[128usize, 128, 0, 0, 0]; 2];
    for y in 0..128 {
        for x in 0..128 {
            if bytes[y * 512 + x * 4] > 25 {
                let b = &mut bounds[usize::from(x >= 64)];
                b[0] = b[0].min(x);
                b[1] = b[1].min(y);
                b[2] = b[2].max(x);
                b[3] = b[3].max(y);
                b[4] += 1;
            }
        }
    }
    for b in bounds {
        assert!(
            b[4] > 150,
            "Each instance must draw its own petal: {bounds:?}"
        );
    }
    assert!(
        bounds[0][3] - bounds[0][1] > bounds[0][2] - bounds[0][0],
        "Unrotated petal should be tall"
    );
    assert!(
        bounds[1][2] - bounds[1][0] > bounds[1][3] - bounds[1][1],
        "90-degree petal should be wide"
    );
    let full_pixels = bounds[0][4] + bounds[1][4];
    petals.set_scale(&device, 0.5).unwrap();
    let scaled = render(&petals);
    let scaled_pixels = scaled.chunks_exact(4).filter(|p| p[0] > 25).count();
    assert!(scaled_pixels > full_pixels / 6 && scaled_pixels < full_pixels / 3);
    // Replacing particle buffers must preserve the global scale uniform.
    petals.reset_particles(&device, &particles).unwrap();
    assert_eq!(render(&petals), scaled);
    for invalid in [0.19, 1.01, f32::NAN, f32::INFINITY] {
        assert!(petals.set_scale(&device, invalid).is_err());
    }
    assert_eq!(render(&petals), scaled);
    petals.set_scale(&device, 0.2).unwrap();
    let smallest_pixels = render(&petals)
        .chunks_exact(4)
        .filter(|p| p[0] > 25)
        .count();
    assert!(smallest_pixels > 0 && smallest_pixels < scaled_pixels / 3);
    petals.set_scale(&device, 1.).unwrap();
    assert_eq!(render(&petals), bytes);
    for p in &mut particles {
        p.appearance[2] = 0.5;
    }
    petals.buffers.states[petals.current].write(&device, &particles, 0);
    let half_pixels = render(&petals)
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 25)
        .count();
    assert!(half_pixels > full_pixels / 6 && half_pixels < full_pixels / 3,
        "Half-size petals should cover about a quarter of the pixels: {half_pixels} vs {full_pixels}");
    for p in &mut particles {
        p.appearance[2] = 0.;
    }
    petals.buffers.states[petals.current].write(&device, &particles, 0);
    assert!(
        render(&petals)
            .chunks_exact(4)
            .all(|pixel| pixel[..3] == [0, 0, 0]),
        "At birth/death zero-scale petals must draw no fragments"
    );
}
