#version 450

#extension GL_ARB_shader_viewport_layer_array : enable

layout(location = 0) out vec3 texCoords;

void main() {
    gl_Position = vec4(
            (float(gl_VertexIndex & 1)) * 4.0 - 1.0,
            (float((gl_VertexIndex >> 1) & 1)) * 4.0 - 1.0,
            0, 
            1.0
        );
    vec2 texCoords2D = gl_Position.xy * 0.5 + 0.5;
    texCoords = vec3(texCoords2D.x, 1 - texCoords2D.y, gl_InstanceIndex);
    gl_Layer = gl_InstanceIndex;
}