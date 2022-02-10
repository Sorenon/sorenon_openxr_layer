#version 450

layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec2 texCoords;

void main() {
    gl_Position = vec4(
            (float(gl_VertexIndex & 1)) * 4.0 - 1.0,
            (float((gl_VertexIndex >> 1) & 1)) * 4.0 - 1.0,
            0, 
            1.0
        );
    texCoords = gl_Position.xy * 0.5 + 0.5;
    texCoords = vec2(texCoords.x, 1 - texCoords.y);
}