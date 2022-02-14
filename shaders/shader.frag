#version 450

layout(location = 0) in vec3 texCoords;

layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2DArray texSampler;

void main() {
    outColor = texture(texSampler, texCoords);
}