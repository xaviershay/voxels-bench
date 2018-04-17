#version 330 core
layout (location = 0) in ivec3 a_pos;

out VS_OUT {
    ivec3 a_pos;
} vs_out;

void main()
{
    gl_Position = vec4(a_pos.x*0.3-0.3, a_pos.y*0.5-0.3, a_pos.z*0.5-0.3, 1.0);
    vs_out.a_pos = a_pos;
}