#version 440

in vec3 posBox;
in vec3 radiusBox;

uniform mat4 mvp;
uniform mat4 invP;
uniform mat4 invMv;
uniform vec2 screenSize;

out vec4 fragColor;

vec3 light = normalize( vec3(0.57703) );

struct Ray {
    vec3 direction;
    vec3 origin;
};

struct Box {
    vec3     center;
    vec3     radius;
    vec3     invRadius;
    mat3     rotation;
};

float maxComponent(vec3 v) { return max (max(v.x, v.y), v.z); }

float safeInverse(float x) { return (x == 0.0) ? 1e12 : (1.0 / x); }
vec3 safeInverse(vec3 v) { return vec3(safeInverse(v.x), safeInverse(v.y), safeInverse(v.z)); }

bool ourHitAABox(vec3 boxCenter, vec3 boxRadius, vec3 rayOrigin, vec3 rayDirection, vec3 invRayDirection) {
    rayOrigin -= boxCenter;
    vec3 distanceToPlane = (-boxRadius * sign(rayDirection) - rayOrigin) * invRayDirection;

#   define TEST(U, V,W)\
         (float(distanceToPlane.U >= 0.0) * \
          float(abs(rayOrigin.V + rayDirection.V * distanceToPlane.U) < boxRadius.V) *\
          float(abs(rayOrigin.W + rayDirection.W * distanceToPlane.U) < boxRadius.W))

    // If the ray is in the box or there is a hit along any axis, then there is a hit
    return bool(float(abs(rayOrigin.x) < boxRadius.x) *
                float(abs(rayOrigin.y) < boxRadius.y) *
                float(abs(rayOrigin.z) < boxRadius.z) +
                TEST(x, y, z) +
                TEST(y, z, x) +
                TEST(z, x, y));
#   undef TEST
}

bool rayBoxIntersect(Box box, Ray ray, out float dist, out vec3 normal,
    const bool canStartInBox, const in bool oriented, in vec3 _invRayDir) {
    ray.origin = ray.origin - box.center;
    if (oriented) {
        ray.origin *= box.rotation;
        ray.direction *= box.rotation;
    }

    float winding = canStartInBox && (maxComponent(abs(ray.origin) * box.invRadius)
        < 1.0) ? -1 : 1;

    vec3 sgn = -sign(ray.direction);

    vec3 d = box.radius * winding * sgn - ray.origin;
    if (oriented) d /= ray.direction; else d *= _invRayDir;

    # define TEST(U, VW)\
        (d.U >= 0.0) && \
        all(lessThan(abs(ray.origin.VW + ray.direction.VW * d.U), box.radius.VW))

    bvec3 test = bvec3(TEST(x, yz), TEST(y, zx), TEST(z, xy));
    sgn = test.x ? vec3(sgn.x, 0.0, 0.0) : (test.y ? vec3(0.0, sgn.y , 0.0) :
        vec3(0.0, 0.0 , test.z ? sgn.z : 0.0));
    # undef TEST

    dist = (sgn.x != 0) ? d.x : ((sgn.y != 0) ? d.y : d.z);
    normal = oriented ? (box.rotation * sgn) : sgn;

    return (sgn.x != 0) || (sgn.y != 0) || (sgn.z != 0);
}


float GetLight(vec3 p, vec3 normal) {
    vec3 lightPos = vec3(16, 16, 16);
    vec3 l = normalize(lightPos-p);

    float dif = clamp(abs(dot(normal, l)), 0.0, 1.0);
    return dif;
}

void main() {

    vec2 uv = 2*((gl_FragCoord.xy + 0.5) / screenSize.xy)-1;
    vec4 osCamPos = invMv * vec4(0,0,0,1);
	vec3 ro = osCamPos.xyz;//ray origin
    vec4 rdh = (invMv * invP) * vec4(uv,-1.0,1.0);
	vec3 rd = normalize((rdh.xyz/rdh.w) - ro);//ray direction

    Ray r = Ray(rd, ro);
    Box b = Box(posBox, radiusBox, safeInverse(radiusBox), mat3(1.0f));

    float dist;
    vec3 normal;

    bool trace = rayBoxIntersect(b, r, dist, normal,
        false, false, safeInverse(r.direction));




    vec3 viewDir =  osCamPos.xyz - vec3(0);
    if (trace == false || dot(viewDir, normal) < 000000.1) {
        discard;
    }

    vec3 pos = r.origin + (dist * r.direction);
    vec4 PClip = mvp * vec4(pos, 1.0);
    float ndc_depth = PClip.z / PClip.w;

    gl_FragDepth = (gl_DepthRange.diff * ndc_depth +
            gl_DepthRange.far + gl_DepthRange.near) / 2.0;

    float dif = GetLight(pos, normal);
    vec3 col = (normalize(pos) * vec3(dif));
    fragColor = vec4(col, 0.1);
}
