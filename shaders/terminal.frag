#version 330 core
out vec4 FragColor;

in vec2 TexCoord;

uniform sampler2D terminalTexture;
uniform float time;
uniform vec2 resolution;

// Terminal effect parameters
const float SCANLINE_INTENSITY = 0.05;
const float GLOW_STRENGTH = 0.25;
const vec3 GLOW_COLOR = vec3(0.0, 1.0, 0.7);  // Cyberpunk green
const float CHROMATIC_ABERRATION = 0.002;

void main() {
    // Basic texture sampling with chromatic aberration
    vec4 baseColor = texture(terminalTexture, TexCoord);
    vec4 rColor = texture(terminalTexture, TexCoord + vec2(CHROMATIC_ABERRATION, 0.0));
    vec4 bColor = texture(terminalTexture, TexCoord - vec2(CHROMATIC_ABERRATION, 0.0));
    
    vec4 color = baseColor;
    color.r = rColor.r;
    color.b = bColor.b;

    // Scanline effect
    float scanline = sin(gl_FragCoord.y * 0.5 + time * 2.0) * SCANLINE_INTENSITY + (1.0 - SCANLINE_INTENSITY);
    color.rgb *= scanline;

    // Add subtle glow to text
    float luminance = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));
    vec3 glow = GLOW_COLOR * luminance * GLOW_STRENGTH;
    color.rgb += glow;

    // Screen flicker
    float flicker = sin(time * 10.0) * 0.02 + 0.98;
    color.rgb *= flicker;

    FragColor = color;
}
