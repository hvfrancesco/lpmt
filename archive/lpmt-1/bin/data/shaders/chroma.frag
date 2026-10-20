//#extension GL_ARB_texture_rectangle : enable

uniform sampler2DRect tex;
uniform float greenscreenR;
uniform float greenscreenG;
uniform float greenscreenB;
uniform float greenscreenT;
uniform float tintR;
uniform float tintG;
uniform float tintB;
float factor;

vec4 color;

void main( void )

{
        //vec2 xy = gl_TexCoord[0].xy;
        //vec4 sample = texture2DRect(tex, xy );
	vec4 col = texture2DRect(tex,gl_TexCoord[0].st);
	vec4 nullo = vec4(0.0,0.0,0.0,0.0);
        //gl_FragColor = sample;
        if((abs(col.x-greenscreenR) < greenscreenT) && (abs(col.y-greenscreenG) < greenscreenT) && (abs(col.z-greenscreenB) < greenscreenT)) {
                //gl_FragColor.a = 0.0;
		factor = 1.0;
        }
	else {factor=0.0;}
	col.x = col.x*tintR;
	col.y = col.y*tintG;
	col.z = col.z*tintB;
	gl_FragColor = mix(col, nullo, factor);
}
