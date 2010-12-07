#include "testApp.h"
#include "stdio.h"
#include <iostream>
#include "ofxSimpleGuiToo.h"


//--------------------------------------------------------------
void testApp::setup()
{
    //we run at 60 fps!
    ofSetVerticalSync(true);



    ttf.loadFont("type/frabk.ttf", 11);
    // set border color for quads in setup mode
    borderColor = 0x666666;
    // starts in quads setup mode
    isSetup = True;
    // starts in windowed mode
    bFullscreen	= 0;
    ofSetWindowShape(800, 600);

    // camera stuff
    camWidth = 320;	// try to grab at this size. 
    camHeight = 240;	
    camGrabber.setVerbose(true);
    camGrabber.initGrabber(camWidth,camHeight);


    // defines the first 4 default quads
    quads[0].setup(0.0,0.0,0.5,0.0,0.5,0.5,0.0,0.5);
    quads[0].quadNumber = 0;
    quads[1].setup(0.5,0.0,1.0,0.0,1.0,0.5,0.5,0.5);
    quads[1].quadNumber = 1;
    quads[2].setup(0.0,0.5,0.5,0.5,0.5,1.0,0.0,1.0);
    quads[2].quadNumber = 2;
    quads[3].setup(0.5,0.5,1.0,0.5,1.0,1.0,0.5,1.0);
    quads[3].quadNumber = 3;
    // define last one as active quad
    activeQuad = 3;
    // number of total quads, to be modified later at each quad insertion
    nOfQuads = 4;



    // gui stuff

    gui.addTitle("show/hide quads");
    // overriding default theme
    gui.config->toggleHeight = 24;
    gui.config->sliderTextHeight = 30;
    gui.config->titleHeight = 30;
    gui.config->fullActiveColor = 0x6B404B;
    // adding controls
    gui.addToggle("quad 0", quads[0].isOn);
    gui.addToggle("quad 1", quads[1].isOn);
    gui.addToggle("quad 2", quads[2].isOn);
    gui.addToggle("quad 3", quads[3].isOn);
    gui.addToggle("quad 4", quads[4].isOn);
    gui.addToggle("quad 5", quads[5].isOn);
    gui.addToggle("quad 6", quads[6].isOn);
    gui.addToggle("quad 7", quads[7].isOn);
    gui.addToggle("quad 8", quads[8].isOn);
    gui.addToggle("quad 9", quads[9].isOn);
    gui.addToggle("quad 10", quads[10].isOn);
    gui.addToggle("quad 11", quads[11].isOn);
    gui.addToggle("quad 12", quads[12].isOn);
    gui.addToggle("quad 13", quads[13].isOn);
    gui.addToggle("quad 14", quads[14].isOn);
    gui.addToggle("quad 15", quads[15].isOn);

    gui.addPage("quad 0");
    gui.addTitle("quad n. 0");
    gui.addToggle("show/hide", quads[0].isOn);
    gui.addToggle("solid bg on/off", quads[0].colorBg);
    gui.addColorPicker("Color", &quads[0].bgColor.r);
    gui.addToggle("cam on/off", quads[0].camBg);
    gui.addSlider("camera mult X", quads[0].camMultX, 0.5, 4.0);
    gui.addSlider("camera mult Y", quads[0].camMultY, 0.5, 4.0);;
    gui.addPage("quad 1");
    gui.addTitle("quad n. 1");
    gui.addToggle("show/hide", quads[1].isOn);
    gui.addToggle("solid bg on/off", quads[1].colorBg);
    gui.addColorPicker("Color", &quads[1].bgColor.r);
    gui.addToggle("cam on/off", quads[1].camBg);
    gui.addSlider("camera mult X", quads[1].camMultX, 0.5, 4.0);
    gui.addSlider("camera mult Y", quads[1].camMultY, 0.5, 4.0);;
    gui.addPage("quad 2");
    gui.addTitle("quad n. 2");
    gui.addToggle("show/hide", quads[2].isOn);
    gui.addToggle("solid bg on/off", quads[2].colorBg);
    gui.addColorPicker("Color", &quads[2].bgColor.r);
    gui.addToggle("cam on/off", quads[2].camBg);
    gui.addSlider("camera mult X", quads[2].camMultX, 0.5, 4.0);
    gui.addSlider("camera mult Y", quads[2].camMultY, 0.5, 4.0);;
    gui.addPage("quad 3");
    gui.addTitle("quad n. 3");
    gui.addToggle("show/hide", quads[3].isOn);
    gui.addToggle("solid bg on/off", quads[3].colorBg);
    gui.addColorPicker("Color", &quads[3].bgColor.r);
    gui.addToggle("cam on/off", quads[3].camBg);
    gui.addSlider("camera mult X", quads[3].camMultX, 0.5, 4.0);
    gui.addSlider("camera mult Y", quads[3].camMultY, 0.5, 4.0);;
    gui.addPage("quad 4");
    gui.addTitle("quad n. 4");
    gui.addToggle("show/hide", quads[4].isOn);
    gui.addToggle("solid bg on/off", quads[4].colorBg);
    gui.addColorPicker("Color", &quads[4].bgColor.r);
    gui.addToggle("cam on/off", quads[4].camBg);
    gui.addSlider("camera mult X", quads[4].camMultX, 0.5, 4.0);
    gui.addSlider("camera mult Y", quads[4].camMultY, 0.5, 4.0);;
    gui.addPage("quad 5");
    gui.addTitle("quad n. 5");
    gui.addToggle("show/hide", quads[5].isOn);
    gui.addToggle("solid bg on/off", quads[5].colorBg);
    gui.addColorPicker("Color", &quads[5].bgColor.r);
    gui.addToggle("cam on/off", quads[5].camBg);
    gui.addSlider("camera mult X", quads[5].camMultX, 0.5, 4.0);
    gui.addSlider("camera mult Y", quads[5].camMultY, 0.5, 4.0);;
    gui.addPage("quad 6");
    gui.addTitle("quad n. 6");
    gui.addToggle("show/hide", quads[6].isOn);
    gui.addToggle("solid bg on/off", quads[6].colorBg);
    gui.addColorPicker("Color", &quads[6].bgColor.r);
    gui.addToggle("cam on/off", quads[6].camBg);
    gui.addSlider("camera mult X", quads[6].camMultX, 0.5, 4.0);
    gui.addSlider("camera mult Y", quads[6].camMultY, 0.5, 4.0);;
    gui.addPage("quad 7");
    gui.addTitle("quad n. 7");
    gui.addToggle("show/hide", quads[7].isOn);
    gui.addToggle("solid bg on/off", quads[7].colorBg);
    gui.addColorPicker("Color", &quads[7].bgColor.r);
    gui.addToggle("cam on/off", quads[7].camBg);
    gui.addSlider("camera mult X", quads[7].camMultX, 0.5, 4.0);
    gui.addSlider("camera mult Y", quads[7].camMultY, 0.5, 4.0);;
    gui.addPage("quad 8");
    gui.addTitle("quad n. 8");
    gui.addToggle("show/hide", quads[8].isOn);
    gui.addToggle("solid bg on/off", quads[8].colorBg);
    gui.addColorPicker("Color", &quads[8].bgColor.r);
    gui.addToggle("cam on/off", quads[8].camBg);
    gui.addSlider("camera mult X", quads[8].camMultX, 0.5, 4.0);
    gui.addSlider("camera mult Y", quads[8].camMultY, 0.5, 4.0);;
    gui.addPage("quad 9");
    gui.addTitle("quad n. 9");
    gui.addToggle("show/hide", quads[9].isOn);
    gui.addToggle("solid bg on/off", quads[9].colorBg);
    gui.addColorPicker("Color", &quads[9].bgColor.r);
    gui.addToggle("cam on/off", quads[9].camBg);
    gui.addSlider("camera mult X", quads[9].camMultX, 0.5, 4.0);
    gui.addSlider("camera mult Y", quads[9].camMultY, 0.5, 4.0);;

    gui.setPage(activeQuad+2);
    gui.show();

}

//--------------------------------------------------------------
void testApp::update()
{
    // grabs video frame from camera
    camGrabber.grabFrame();
    if (camGrabber.isFrameNew()){
		int totalPixels = camWidth*camHeight*3;
		unsigned char * pixels = camGrabber.getPixels();
		for (int i = 0; i < 16; i++) {
			if (quads[i].initialized) {
				if (quads[i].camBg) {
				quads[i].camTexture.loadData(pixels, camWidth,camHeight, GL_RGB);
				quads[i].camWidth = camWidth;
				quads[i].camHeight = camHeight;
				}
			}
		}
	}

    // sets default window background and fixed shape
    if (isSetup) {
    ofBackground(20, 20, 20);
    }
    else {
    ofBackground(0, 0, 0);
    }
    //ofSetWindowShape(800, 600);
    // loops through initialized quads and runs update, setting the border color as well
    for(int i = 0; i < 16; i++)
    {
        if (quads[i].initialized)
        {
            quads[i].update();
            quads[i].borderColor = borderColor;
        }
    }

}

//--------------------------------------------------------------
void testApp::draw()
{

    // in setup mode sets active quad border to be white
    if (isSetup)
    {
        quads[activeQuad].borderColor = 0xFFFFFF;
    }

    // loops through initialized quads and calls their draw function
    for(int i = 0; i < 16; i++)
    {
        if (quads[i].initialized)
        {
            quads[i].draw();
        }
    }



    // in setup mode writes the number of active quad at the bottom of the window
    if (isSetup)
    {
        ofSetColor(0xFFFFFF);
        ttf.drawString("active quad: "+ofToString(activeQuad), 30, ofGetHeight()-25);
    }

   // draws gui
   if (isSetup)
   {
   gui.draw();
   }

}

//--------------------------------------------------------------
void testApp::keyPressed(int key)
{
    // fills window with active quad
    if ( key =='r' || key == 'R' )
    {
        if (isSetup)
        {
            quads[activeQuad].corners[0].x = 0.0;
            quads[activeQuad].corners[0].y = 0.0;

            quads[activeQuad].corners[1].x = 1.0;
            quads[activeQuad].corners[1].y = 0.0;

            quads[activeQuad].corners[2].x = 1.0;
            quads[activeQuad].corners[2].y = 1.0;

            quads[activeQuad].corners[3].x = 0.0;
            quads[activeQuad].corners[3].y = 1.0;
        }
    }
    
    // activates next quad
    if ( key =='>' )
    {
        if (isSetup)
        {
            activeQuad += 1;
            if (activeQuad > nOfQuads-1)
            {
                activeQuad = 0;
            }
        }
    gui.setPage(activeQuad+2);
    }

    // activates prev quad
    if ( key =='<' )
    {
        if (isSetup)
        {
            activeQuad -= 1;
            if (activeQuad < 0)
            {
                activeQuad = nOfQuads-1;
            }
        }
    gui.setPage(activeQuad+2);
    }

    // add a new quad
    if ( key =='a' )
    {
        if (isSetup)
        {
            if (nOfQuads < 15)
            {
                quad q;
                quads[nOfQuads].setup(0.25,0.25,0.75,0.25,0.75,0.75,0.25,0.75);
                quads[nOfQuads].quadNumber = nOfQuads;
                activeQuad = nOfQuads;
                ++nOfQuads;
		gui.setPage(activeQuad+2);
            }
        }
    }

    // toggles setup mode
    if ( key ==' ' )
    {
        if (isSetup)
        {
            isSetup = False;
            for(int i = 0; i < 16; i++)
            {
                if (quads[i].initialized)
                {
                    quads[i].isSetup = False;
                }
            }
        }
        else
        {
            isSetup = True;
            for(int i = 0; i < 16; i++)
            {
                if (quads[i].initialized)
                {
                    quads[i].isSetup = True;
                }
            }
        }
    }

    // toggles fullscreen mode
    if(key == 'f')
    {

        bFullscreen = !bFullscreen;

        if(!bFullscreen)
        {
            ofSetWindowShape(800, 600);
            ofSetFullscreen(false);
            // figure out how to put the window in the center:
            int screenW = ofGetScreenWidth();
            int screenH = ofGetScreenHeight();
            ofSetWindowPosition(screenW/2-800/2, screenH/2-600/2);
        }
        else if(bFullscreen == 1)
        {
            ofSetFullscreen(true);
        }
    }

    // toggles gui
    if(key == 'g')
    {
    gui.toggleDraw();
    }

    if(key == '[')
    {
    gui.prevPage();
    }

    if(key == ']')
    {
    gui.nextPage();
    }

    if(key == '1')
    {
    gui.setPage(1);
    }

}

//--------------------------------------------------------------
void testApp::keyReleased(int key)
{
}


//--------------------------------------------------------------
void testApp::mouseMoved(int x, int y )
{


}

//--------------------------------------------------------------
void testApp::mouseDragged(int x, int y, int button)
{
    if (isSetup)
    {

        float scaleX = (float)x / ofGetWidth();
        float scaleY = (float)y / ofGetHeight();

        if(whichCorner >= 0)
        {
            quads[activeQuad].corners[whichCorner].x = scaleX;
            quads[activeQuad].corners[whichCorner].y = scaleY;
        }

    }
}

//--------------------------------------------------------------
void testApp::mousePressed(int x, int y, int button)
{
    if (isSetup)
    {
        float smallestDist = 1.0;
        whichCorner = -1;

        for(int i = 0; i < 4; i++)
        {
            float distx = quads[activeQuad].corners[i].x - (float)x/ofGetWidth();
            float disty = quads[activeQuad].corners[i].y - (float)y/ofGetHeight();
            float dist  = sqrt( distx * distx + disty * disty);

            if(dist < smallestDist && dist < 0.1)
            {
                whichCorner = i;
                smallestDist = dist;
            }
        }
    }
}

//--------------------------------------------------------------
void testApp::mouseReleased()
{
    whichCorner = -1;
}


