void main() {
	gl_TexCoord[0] = gl_MultiTexCoord0;
	gl_Position = ftransform();
	vec4 col = gl_Color;
	gl_FrontColor =  col;
} 
