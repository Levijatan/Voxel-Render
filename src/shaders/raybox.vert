#version 440

layout (location = 0) in vec3 vPos;
layout (location = 1) in float lightLevel;


uniform mat4 mvp;
uniform vec2 screenSize;
uniform float voxelSize;

out vec3 posBox;
out vec3 radiusBox;
out float light;


void quadricProj(in vec3 osPosition, in float voxelSize,
    in mat4 objectToScreenMatrix, in vec2 halfScreenSize, inout vec4 position,
    inout float pointSize) {

    const vec4 quadricMat = vec4(1.0, 1.0, 1.0, -1.0);
    float sphereRadius = voxelSize * 1.732051;
    vec4 sphereCenter = vec4(osPosition.xyz, 1.0);
    mat4 modelViewProj = transpose(objectToScreenMatrix);

    mat3x4 matT = mat3x4(mat3(modelViewProj[0].xyz, modelViewProj[1].xyz,
        modelViewProj[3].xyz) * sphereRadius);
    matT[0].w = dot(sphereCenter, modelViewProj[0]);
    matT[1].w = dot(sphereCenter, modelViewProj[1]);
    matT[2].w = dot(sphereCenter, modelViewProj[3]);

    mat3x4 matD = mat3x4(matT[0] * quadricMat, matT[1] * quadricMat,
        matT[2] * quadricMat);
    vec4 eqCoefs = vec4(dot(matD[0], matT[2]), dot(matD[1], matT[2]),
        dot(matD[0], matT[0]), dot(matD[1], matT[1])) / dot(matD[2], matT[2]);

    vec4 outPosition = vec4(eqCoefs.x, eqCoefs.y, 0.0, 1.0);
    vec2 AABB = sqrt(eqCoefs.xy*eqCoefs.xy - eqCoefs.zw);
    AABB *= halfScreenSize * 2.0f;

    position.xy = outPosition.xy * position.w;
    pointSize = max(AABB.x, AABB.y);
}



void main() {
    vec3 vertex = vPos;
    vec4 position = mvp * vec4(vertex, 1);
    float pointSize;

    quadricProj(vertex, voxelSize, mvp, screenSize/2.0, position, pointSize);

    float stochasticCoverage = pointSize * pointSize;
    if ((stochasticCoverage < 0.8) &&
        ((gl_VertexID & 0xffff) > stochasticCoverage * (0xffff / 0.8))) {
            // "Cull" small voxels in a stable, stochastic way by moving past the z = 0 plane.
            // Assumes voxels are in randomized order.
        position = vec4(-1,-1,-1,-1);
    }

    gl_Position = position;
    gl_PointSize = pointSize;

    posBox = vertex;
    radiusBox = vec3(voxelSize/2);
    light = lightLevel;
}
