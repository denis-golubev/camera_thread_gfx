#version 150 core

in vec2 pos;
in vec2 texture_pos;

out vec2 v_texture_pos;

void main() {
    gl_Position = vec4(pos, 0.0, 1.0);
    v_texture_pos = texture_pos;
}
