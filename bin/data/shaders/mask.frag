//#version 140


uniform sampler2DRect tex;
uniform sampler2DRect mask;
uniform int mode;
float factor;

//uniform int w;
//uniform int h;

void main(){
	//this is the fragment shader
	//this is where the pixel level drawing happens
	//gl_FragCoord gives us the x and y of the current pixel its drawing

	vec4 col = texture2DRect(tex,gl_TexCoord[0].st);
	vec4 maskcol = texture2DRect(mask,gl_TexCoord[0].st);
	vec4 nullo = vec4(0.0,0.0,0.0,0.0);
	//if(mode == 0) factor = maskcol.w;
	//if(mode == 1) factor = 1.0 - maskcol.w;
    //if(mode == 0) factor = (maskcol.x*maskcol.y*maskcol.z*maskcol.w);
	//if(mode == 1) factor = 1.0 - (maskcol.x*maskcol.y*maskcol.z*maskcol.w);
    if(mode == 0) factor = ((maskcol.x+maskcol.y+maskcol.z)/3.0*maskcol.w);
	if(mode == 1) factor = 1.0 - ((maskcol.x+maskcol.y+maskcol.z)/3.0*maskcol.w);

	// set final color
    //gl_FragData[0] = vec4(col.xyzw);
    gl_FragColor = mix(col, nullo, factor);


}
