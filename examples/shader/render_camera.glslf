#version 150 core

in vec2 v_texture_pos;

uniform sampler2D texture_camera;

out vec4 screen;

void main() {
    screen = texture(texture_camera, v_texture_pos);
//    screen = vec4(screen.x + screen.y + screen.z, 0.0, 1.0, 1.0);
//    screen = vec4(v_texture_pos, screen.z, 1.0);
}
