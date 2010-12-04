//from http://groups.google.com/group/comp.graphics.algorithms/browse_thread/thread/da41b1da909bc91f/303c5f533bb4b8a6

uniform float testX;
uniform float testY;

uniform float Q1x;
uniform float Q1y;

uniform float Q2x;
uniform float Q2y;

uniform float Q3x;
uniform float Q3y;

uniform float Q4x;
uniform float Q4y;

uniform float totalWidth;
uniform float totalHeight;

float bx; 
float ax;

float by; 
float ay;

float C1x, C1y, C2x, C2y;

void main()
{
	gl_FrontColor = gl_Color;
	vec4 v = vec4(gl_Vertex);

	// X fractions 
	bx = v.x / totalWidth; 
	ax = 1.0 - bx; 

	// Helper points: 
	C1x = Q1x * ax + Q2x * bx; 
	C1y = Q1y * ax + Q2y * bx; 
	C2x = Q3x * ax + Q4x * bx; 
	C2y = Q3y * ax + Q4y * bx; 
 
	//Y fractions 
     by = v.y / totalHeight; 
     ay = 1.0 - by; 

	v.x = (C1x * ay + C2x * by) * totalWidth; 
	v.y = (C1y * ay + C2y * by) * totalHeight; 
	
	gl_Position 		= gl_ModelViewProjectionMatrix * v;
} 

