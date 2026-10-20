struct Camera {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    // Width, height, sRGB render attachment, padding.
    viewport: vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var positions: texture_2d<f32>;
@group(0) @binding(2) var attributes: texture_2d<u32>;

const NEAR_HIDE_DISTANCE: f32 = 0.5;
const NEAR_VISIBLE_DISTANCE: f32 = 1.0;

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) gaussian: vec2<f32>,
    @location(1) color: vec4<f32>,
};

fn project(vertex: u32, index: u32, depth_only: bool) -> Vertex {
    var out: Vertex;
    out.position = vec4<f32>(2.0, 2.0, 2.0, 1.0);
    let size = textureDimensions(positions);
    let uv = vec2<i32>(i32(index % size.x), i32(index / size.x));
    let p = textureLoad(positions, uv, 0);
    let center = camera.view * vec4<f32>(p.xyz, 1.0);
    let depth = -center.z;
    if depth <= NEAR_HIDE_DISTANCE { return out; }
    let clip = camera.projection * center;
    if (clip.w <= 0.0 || clip.z < 0.0 || clip.z > clip.w) { return out; }

    let packed = textureLoad(attributes, uv, 0);
    var color = unpack4x8unorm(packed.w);
    // Fade color and occlusion together so hidden foreground leaves no invisible wall.
    color.a *= smoothstep(NEAR_HIDE_DISTANCE, NEAR_VISIBLE_DISTANCE, depth);
    if depth_only && color.a < 0.3 { return out; }
    let xy = unpack2x16float(packed.x);
    let zw = unpack2x16float(packed.y);
    let uv_cov = unpack2x16float(packed.z);
    let covariance = mat3x3<f32>(
        vec3<f32>(xy.x, xy.y, zw.x),
        vec3<f32>(xy.y, zw.y, uv_cov.x),
        vec3<f32>(zw.x, uv_cov.x, uv_cov.y)
    ) * p.w;
    let focal = vec2<f32>(camera.projection[0][0], camera.projection[1][1]) * camera.viewport.xy * 0.5;
    // Limit the off-axis Jacobian for Gaussians intersecting the viewport edges.
    let slope = clamp(center.xy / depth, -1.3 / vec2<f32>(camera.projection[0][0], camera.projection[1][1]), 1.3 / vec2<f32>(camera.projection[0][0], camera.projection[1][1]));
    let rotation = mat3x3<f32>(camera.view[0].xyz, camera.view[1].xyz, camera.view[2].xyz);
    let jx = transpose(rotation) * vec3<f32>(focal.x / depth, 0.0, focal.x * slope.x / depth);
    let jy = transpose(rotation) * vec3<f32>(0.0, focal.y / depth, focal.y * slope.y / depth);
    let a = dot(jx, covariance * jx) + 0.3;
    let b = dot(jx, covariance * jy);
    let c = dot(jy, covariance * jy) + 0.3;
    let mean = (a + c) * 0.5;
    let spread = length(vec2<f32>((a - c) * 0.5, b));
    let eigenvalues = max(vec2<f32>(mean + spread, mean - spread), vec2<f32>(0.1));
    var major = vec2<f32>(1.0, 0.0);
    if (abs(b) > 0.000001) { major = normalize(vec2<f32>(b, eigenvalues.x - a)); }
    else if (c > a) { major = vec2<f32>(0.0, 1.0); }
    let minor = vec2<f32>(-major.y, major.x);
    let corners = array<vec2<f32>, 4>(vec2<f32>(-1,-1),vec2<f32>(1,-1),vec2<f32>(-1,1),vec2<f32>(1,1));
    var extent = 3.0;
    if depth_only {
        // Bound the existing alpha >= 0.3 contour, with conservative rounding padding.
        let alpha = color.a;
        extent = min(3.0, sqrt(max(0.0, 2.0 * log(max(alpha, 0.000001) / 0.3))) + 0.001);
    }
    let local = corners[vertex] * extent;
    let offset = major * sqrt(eigenvalues.x) * local.x + minor * sqrt(eigenvalues.y) * local.y;
    out.position = vec4<f32>(clip.xy + offset * (2.0 / camera.viewport.xy) * clip.w, clip.zw);
    out.gaussian = local;
    out.color = color;
    if (camera.viewport.z > 0.5) {
        // SH DC is display RGB; avoid applying the attachment's sRGB curve twice.
        let rgb = out.color.rgb;
        out.color = vec4<f32>(select(rgb / 12.92, pow((rgb + 0.055) / 1.055, vec3<f32>(2.4)), rgb > vec3<f32>(0.04045)), out.color.a);
    }
    return out;
}

@vertex fn vs_main(@builtin(vertex_index) vertex: u32, @location(0) index: u32) -> Vertex {
    return project(vertex, index, false);
}

@vertex fn vs_depth(@builtin(vertex_index) vertex: u32, @location(0) index: u32) -> Vertex {
    return project(vertex, index, true);
}

fn shade(in: Vertex) -> vec4<f32> {
    let radius_squared = dot(in.gaussian, in.gaussian);
    let alpha = min(0.99, in.color.a * exp(-0.5 * radius_squared));
    if (radius_squared > 9.0 || alpha < 1.0 / 255.0) { discard; }

    return vec4<f32>(in.color.rgb * alpha, alpha);
}

@fragment fn fs_depth(in: Vertex) {
    let r2 = dot(in.gaussian, in.gaussian);
    let alpha = in.color.a * exp(-0.5 * r2);
    // Only substantial coverage occludes the emissive particles. Low-opacity
    // Gaussian tails must not turn into large invisible walls.
    if r2 > 9.0 || alpha < 0.3 { discard; }
}

@fragment fn fs_main(in: Vertex) -> @location(0) vec4<f32> {
    return shade(in);
}
struct ForestOutput {
    @location(0) color: vec4<f32>,
    @location(1) occlusion: f32,
};
@fragment fn fs_merged(in: Vertex) -> ForestOutput {
    let color = shade(in);
    // Min blending is the same nearest-depth reduction as the separate depth pass.
    return ForestOutput(color, select(1.0, in.position.z, color.a >= 0.3));
}
