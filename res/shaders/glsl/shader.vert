#version 450

// Camera View & Projection
layout(binding = 0) uniform camera_vp {
    mat4 view;
    mat4 proj;
} camera;

// Model Instance Transform
layout( push_constant ) uniform model_transform {
	mat4 transform;
} model;

// Vertex Properties
layout(location = 0) in vec3 vertex_position;
layout(location = 1) in vec3 vertex_color;

layout(location = 0) out vec3 out_color;

void main() {
    gl_Position = camera.proj * camera.view * model.transform * vec4(vertex_position, 1.0);
    out_color = vertex_color;
}