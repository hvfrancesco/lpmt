//#version 140


//uniform float time;
uniform sampler2DRect tex;

// used in a few inversions
const vec3 one = vec3(1.0);
//const vec3 gamma = vec3(1.8, 1.5, 1.2);
//const vec3 luminance = vec3(0.5);

// controls the interpolation curve ([1..n], 1.0 = linear, 2.0 = default quadratic)
uniform float exponent; // try: 2.0;
// controls the center of interpolation ([0..1], 0.5 = linear)
uniform vec3 userLuminance; // try: vec3(0.5);
// controls gamma levels ([1..n], 1.8 or 2.2 is typical)
//uniform vec3 gamma; // try: vec3(1.8, 1.5, 1.2);
// controls blending area at left, top, right and bottom in percentages ([0..0.5])
uniform vec4 amount;
//uniform vec4 edges; // try: vec4(0.4, 0.4, 0.0, 0.0);

uniform int w;
uniform int h;
uniform float userGamma;

void main(){
	//this is the fragment shader
	//this is where the pixel level drawing happens
	//gl_FragCoord gives us the x and y of the current pixel its drawing

	//we grab the x and y and store them in an int

	vec2 uv = gl_TexCoord[0].xy;
	//vec4 col = texture2D(tex,uv).xyz;
	vec4 col = texture2DRect(tex,gl_TexCoord[0].st);
	//vec2 uv = vec2(gl_FragCoord.x, gl_FragCoord.y);
    	//vec4 col = gl_Color;
        vec4 edges = vec4(amount.x, amount.y, amount.z, amount.w);
        //vec3 gamma = vec3(userGamma, 1.5, 1.2);
	vec3 gamma = vec3(userGamma, userGamma, userGamma);
	vec3 luminance = vec3(userLuminance + 0.5);




    // calculate edge blending factor
    float a = 1.0;

    if(edges.x > 0.0) a *= clamp((uv.x/float(w))/edges.x, 0.0, 1.0);
	if(edges.y > 0.0) a *= clamp((uv.y/float(h))/edges.y, 0.0, 1.0);
	if(edges.z > 0.0) a *= clamp((1.0-(uv.x/float(w)))/edges.z, 0.0, 1.0);
	if(edges.w > 0.0) a *= clamp((1.0-(uv.y/float(h)))/edges.w, 0.0, 1.0);

	// blend function with luminance control (for each of the 3 channels)
	vec3 blend = (a < 0.5) ? (luminance * pow(2.0 * a, exponent))
		: one - (one - luminance) * pow(2.0 * (1.0 - a), exponent);

	// gamma correction (for each of the 3 channels)
	blend = pow(blend, one / gamma);

	// set final color
    //gl_FragColor = vec4(col * blend, 1.0);
    //gl_FragColor = vec4(col , 1.0);

    //gl_FragColor = vec4(col.xyz*blend, col.w*blend.x);
    gl_FragData[0] = vec4(col.xyz*blend, col.w*blend.x);


}
