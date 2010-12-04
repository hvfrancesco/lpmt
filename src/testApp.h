#ifndef _TEST_APP
#define _TEST_APP

#include "ofMain.h"
#include "ofAddons.h"
#include "ball.h"
#include "quad.h"


class testApp : public ofSimpleApp{

	public:

		void setup();
		void update();
		void draw();

		void keyPressed(int key);
		void keyReleased(int key);
		void mouseMoved(int x, int y );
		void mouseDragged(int x, int y, int button);
		void mousePressed(int x, int y, int button);
		void mouseReleased();
		int whichCorner;
		ofTrueTypeFont ttf;

		quad quads[16];
		
		int activeQuad;
		int nOfQuads;
		int borderColor;
		
		bool isSetup;
		bool bFullscreen;
		

};

#endif

