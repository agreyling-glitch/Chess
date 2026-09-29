struct SceneInput {
    @location(0) screen_depth_mode: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal: vec4<f32>,
    @location(4) tangent: vec4<f32>,
    @location(5) bitangent: vec4<f32>,
    @location(6) world: vec4<f32>,
};
struct SceneOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tangent: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) world: vec3<f32>,
    @location(6) @interpolate(flat) mode: u32,
};
@group(0) @binding(0) var white_diff: texture_2d<f32>;
@group(0) @binding(1) var black_diff: texture_2d<f32>;
@group(0) @binding(2) var board_diff: texture_2d<f32>;
@group(0) @binding(3) var board_normal: texture_2d<f32>;
@group(0) @binding(4) var board_arm: texture_2d<f32>;
@group(0) @binding(5) var white_normal: texture_2d<f32>;
@group(0) @binding(6) var black_normal: texture_2d<f32>;
@group(0) @binding(7) var texture_sampler: sampler;
@group(0) @binding(8) var<uniform> camera_adjustment: vec4<f32>;

@vertex fn scene_vertex(input: SceneInput) -> SceneOutput {
    var out: SceneOutput;
    let original_depth = input.screen_depth_mode.z;
    // Zoom moves the eye along the unchanged view direction. The projected
    // x/y numerator stays fixed; only perspective depth changes.
    let depth = original_depth + camera_adjustment.x;
    let ndc = vec2<f32>(input.screen_depth_mode.x * 2.0 - 1.0, 1.0 - input.screen_depth_mode.y * 2.0);
    out.clip = vec4<f32>(ndc * original_depth, depth - 0.1, depth);
    out.uv = input.uv;
    out.color = input.color;
    out.normal = input.normal.xyz;
    out.tangent = input.tangent.xyz;
    out.bitangent = input.bitangent.xyz;
    out.world = input.world.xyz;
    out.mode = u32(input.screen_depth_mode.w);
    return out;
}

fn lighting(normal: vec3<f32>, world: vec3<f32>, black: bool) -> vec2<f32> {
    let wood = u32(camera_adjustment.z) == 1u;
    let key = normalize(select(vec3<f32>(-0.55, 1.0, 0.75), vec3<f32>(-0.25, 1.0, 0.05), wood));
    let fill = normalize(vec3<f32>(0.8, 0.55, -0.35));
    let view = normalize(vec3<f32>(0.0, 13.0, 11.0) - world);
    let ambient = select(select(0.32, 0.39, black), select(0.50, 0.62, black), wood);
    let key_power = select(0.43, 0.25, wood);
    let fill_power = select(select(0.17, 0.29, black), 0.20, wood);
    let ceiling = select(0.92, select(1.08, 1.12, wood), black);
    let center_light = select(1.0, clamp(1.04 - 0.003 * dot(world.xz, world.xz), 0.96, 1.04), wood);
    let diffuse = clamp((ambient + key_power * max(dot(normal, key), 0.0) + fill_power * max(dot(normal, fill), 0.0) + 0.08 * max(normal.y, 0.0)) * center_light, 0.32, ceiling);
    let kr = max(dot(normal, normalize(key + view)), 0.0);
    let fr = max(dot(normal, normalize(fill + view)), 0.0);
    let reflection = select(28.0 * pow(kr, 12.0) + 36.0 * pow(kr, 48.0) + 18.0 * pow(fr, 16.0),
        22.0 * pow(kr, 20.0) + 45.0 * pow(kr, 48.0) + 5.0 * pow(fr, 32.0), wood);
    return vec2<f32>(diffuse, reflection * center_light);
}

fn appearance_strength() -> f32 {
    return clamp((camera_adjustment.y - 0.5) * 2.0, -1.0, 1.0);
}

fn appearance_color(color: vec3<f32>) -> vec3<f32> {
    let strength = appearance_strength();
    let contrast = 1.0 + 0.2 * strength;
    let brightness = 0.08 * strength;
    return clamp((color - vec3<f32>(0.42)) * contrast + vec3<f32>(0.42 + brightness), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment fn scene_fragment(input: SceneOutput) -> @location(0) vec4<f32> {
    if (input.mode == 5u) {
        let radial = 2.0 * input.uv - vec2<f32>(1.0);
        let alpha = 0.55 * pow(max(1.0 - dot(radial, radial), 0.0), 2.0);
        if (alpha <= 0.001) { discard; }
        return vec4<f32>(input.color.rgb, alpha);
    }
    if (input.mode == 4u) {
        let radius_sq = dot(2.0 * input.uv - vec2<f32>(1.0), 2.0 * input.uv - vec2<f32>(1.0));
        let alpha = 0.34 * pow(max(1.0 - radius_sq, 0.0), 2.0);
        if (alpha <= 0.001) { discard; }
        return vec4<f32>(0.0, 0.0, 0.0, alpha);
    }
    if (input.mode == 0u) { return input.color; }
    let base_normal = normalize(input.normal);
    let tangent = normalize(input.tangent - base_normal * dot(input.tangent, base_normal));
    let handedness = select(-1.0, 1.0, dot(cross(base_normal, tangent), input.bitangent) >= 0.0);
    let bitangent = cross(base_normal, tangent) * handedness;
    var map = vec3<f32>(0.5, 0.5, 1.0);
    var strength = 0.55;
    if (input.mode == 1u) {
        map = textureSampleLevel(board_normal, texture_sampler, input.uv, 0.0).rgb;
        strength = 0.7;
    } else if (input.mode == 2u) {
        map = textureSampleLevel(white_normal, texture_sampler, input.uv, 0.0).rgb;
    } else {
        map = textureSampleLevel(black_normal, texture_sampler, input.uv, 0.0).rgb;
    }
    let mapped = map * 2.0 - vec3<f32>(1.0);
    let theme = u32(camera_adjustment.z);
    let normal = select(normalize(base_normal * max(mapped.z, 0.1) + tangent * mapped.x * strength + bitangent * mapped.y * strength),
        base_normal, theme >= 3u && input.mode >= 2u);
    let lit = lighting(normal, input.world, input.mode == 3u);
    if (theme == 2u && input.mode >= 2u) {
        let view = normalize(vec3<f32>(0.0, 13.0, 11.0) - input.world);
        let rim = pow(1.0 - abs(dot(base_normal, view)), 2.5);
        let tint = select(vec3<f32>(0.59, 0.79, 0.86), vec3<f32>(0.66, 0.20, 0.44), input.mode == 3u);
        let rim_color = select(vec3<f32>(0.48, 0.52, 0.54), vec3<f32>(0.50, 0.24, 0.41), input.mode == 3u);
        let color = min(tint * (0.42 + 0.25 * lit.x) + rim_color * rim
            + vec3<f32>(lit.y * 1.5 / 255.0), vec3<f32>(0.98));
        return vec4<f32>(appearance_color(color), 1.0);
    }
    if (input.mode == 1u) {
        let marble = textureSampleLevel(board_diff, texture_sampler, input.uv, 0.0).rgb;
        let roughness = textureSampleLevel(board_arm, texture_sampler, input.uv, 0.0).g;
        let gloss = select(clamp(0.42 + 1.18 * (1.0 - roughness), 0.42, 1.35),
            clamp(0.55 + 0.8 * (1.0 - roughness), 0.55, 1.35), theme == 1u) * (1.0 + 0.75 * appearance_strength());
        let wood_gray = dot(marble, vec3<f32>(0.2126, 0.7152, 0.0722));
        let quiet_wood = mix(vec3<f32>(wood_gray), marble, 0.12) * 0.72;
        let base = select(select(marble, quiet_wood, theme == 1u),
            marble * vec3<f32>(0.40, 0.52, 0.63) + vec3<f32>(0.08, 0.14, 0.19), theme == 2u);
        let color = min(base * lit.x + vec3<f32>(lit.y * gloss / 255.0), vec3<f32>(230.0 / 255.0));
        return vec4<f32>(appearance_color(color), 1.0);
    }
    if (theme >= 3u) {
        let paint = select(textureSampleLevel(white_diff, texture_sampler, input.uv, 0.0).rgb,
            textureSampleLevel(black_diff, texture_sampler, input.uv, 0.0).rgb, input.mode == 3u);
        let color = min(paint * lit.x + vec3<f32>(lit.y * 0.6 / 255.0), vec3<f32>(0.98));
        return vec4<f32>(appearance_color(color), 1.0);
    }
    if (theme == 1u) {
        let wood = select(textureSampleLevel(white_diff, texture_sampler, input.uv, 0.0).rgb,
            textureSampleLevel(black_diff, texture_sampler, input.uv, 0.0).rgb, input.mode == 3u);
        let scale = select(vec3<f32>(0.99, 0.98, 0.96), vec3<f32>(0.72, 0.74, 0.76), input.mode == 3u);
        let lift = select(6.0, 3.0, input.mode == 3u) / 255.0;
        let color = min((wood * scale + vec3<f32>(lift)) * lit.x + vec3<f32>(lit.y * 0.35 * (1.0 + 0.75 * appearance_strength()) / 255.0), vec3<f32>(245.0 / 255.0));
        return vec4<f32>(appearance_color(color), 1.0);
    }
    var detail = 0.0;
    var ceiling = 230.0 / 255.0;
    if (input.mode == 2u) {
        detail = dot(textureSampleLevel(white_diff, texture_sampler, input.uv, 0.0).rgb, vec3<f32>(0.2126, 0.7152, 0.0722)) * 255.0 / 210.0;
        ceiling = 245.0 / 255.0;
    } else {
        detail = dot(textureSampleLevel(black_diff, texture_sampler, input.uv, 0.0).rgb, vec3<f32>(0.2126, 0.7152, 0.0722)) * 255.0 / 48.0;
    }
    let brightness = (0.78 + 0.22 * clamp(detail, 0.0, 1.35)) * lit.x;
    let specular = lit.y * (1.0 + 0.75 * appearance_strength());
    let color = min(input.color.rgb * brightness + vec3<f32>(specular, specular * 0.97, specular * 0.93) / 255.0, vec3<f32>(ceiling));
    return vec4<f32>(appearance_color(color), 1.0);
}

struct BlitOutput { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn blit_vertex(@builtin(vertex_index) index: u32) -> BlitOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0));
    let p = corners[index];
    var out: BlitOutput;
    out.clip = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5);
    return out;
}
@group(0) @binding(0) var rendered_board: texture_2d<f32>;
@group(0) @binding(1) var rendered_sampler: sampler;
@fragment fn blit_fragment(input: BlitOutput) -> @location(0) vec4<f32> {
    let encoded = textureSampleLevel(rendered_board, rendered_sampler, input.uv, 0.0).rgb;
    return vec4<f32>(encoded, 1.0);
}
@fragment fn blit_fragment_srgb(input: BlitOutput) -> @location(0) vec4<f32> {
    let encoded = textureSampleLevel(rendered_board, rendered_sampler, input.uv, 0.0).rgb;
    return vec4<f32>(pow(encoded, vec3<f32>(2.2)), 1.0);
}
