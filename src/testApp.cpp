#include "testApp.h"
#include "stdio.h"
#include <iostream>
#include "ofxSimpleGuiToo.h"

#include <sys/types.h>
#include <dirent.h>
#include <errno.h>
#include <vector>
#include <string>


using namespace std;

int getdir (string dir, vector<string> &files)
{
    DIR *dp;
    struct dirent *dirp;
    if((dp  = opendir(dir.c_str())) == NULL) {
        cout << "Error(" << errno << ") opening " << dir << endl;
        return errno;
    }

    while ((dirp = readdir(dp)) != NULL) {
        files.push_back(string(dirp->d_name));
    }
    closedir(dp);
    return 0;
}



//--------------------------------------------------------------
void testApp::setup()
{
    //we run at 60 fps!
    ofSetVerticalSync(true);

    // we scan the img dir for images
	string imgDir = string("./data/img");
    imgFiles = vector<string>();
    getdir(imgDir,imgFiles);
    string images[imgFiles.size()];
    for (unsigned int i = 0;i < imgFiles.size();i++) {
        images[i]= imgFiles[i];
    }


    // we scan the video dir for videos
	string videoDir = string("./data/video");
    videoFiles = vector<string>();
    getdir(videoDir,videoFiles);
    string videos[videoFiles.size()];
    for (unsigned int i = 0;i < videoFiles.size();i++) {
        videos[i]= videoFiles[i];
    }

    // we scan the slideshow dir for videos
	string slideshowDir = string("./data/slideshow");
    slideshowFolders = vector<string>();
    getdir(slideshowDir,slideshowFolders);
    string slideshows[slideshowFolders.size()];
    for (unsigned int i = 0;i < slideshowFolders.size();i++) {
        slideshows[i]= slideshowFolders[i];
    }


    ttf.loadFont("type/frabk.ttf", 11);
    // set border color for quads in setup mode
    borderColor = 0x666666;
    // starts in quads setup mode
    isSetup = True;
    // starts in windowed mode
    bFullscreen	= 0;
    // gui is on at start
    bGui = 1;
    ofSetWindowShape(800, 600);

    // camera stuff
    camWidth = 640;	// try to grab at this size.
    camHeight = 480;
    camGrabber.setVerbose(true);
    camGrabber.initGrabber(camWidth,camHeight);

    // texture for snapshot background
    snapshotTexture.allocate(camWidth,camHeight, GL_RGB);
    snapshotOn = 0;


    // defines the first 4 default quads
    quads[0].setup(0.0,0.0,0.5,0.0,0.5,0.5,0.0,0.5,imgFiles, videoFiles, slideshowFolders);
    quads[0].quadNumber = 0;
    quads[1].setup(0.5,0.0,1.0,0.0,1.0,0.5,0.5,0.5,imgFiles, videoFiles, slideshowFolders);
    quads[1].quadNumber = 1;
    quads[2].setup(0.0,0.5,0.5,0.5,0.5,1.0,0.0,1.0,imgFiles, videoFiles, slideshowFolders);
    quads[2].quadNumber = 2;
    quads[3].setup(0.5,0.5,1.0,0.5,1.0,1.0,0.5,1.0,imgFiles, videoFiles, slideshowFolders);
    quads[3].quadNumber = 3;
    // define last one as active quad
    activeQuad = 3;
    // number of total quads, to be modified later at each quad insertion
    nOfQuads = 4;


    // gui stuff

    gui.addTitle("show/hide quads");
    // overriding default theme
    gui.config->toggleHeight = 20;
    gui.config->sliderTextHeight = 24;
    gui.config->titleHeight = 20;
    gui.config->fullActiveColor = 0x6B404B;

    // adding controls
    // first a general page for toggling layers on/off
    for(int i = 0; i < 20; i++)
    {
    gui.addToggle("quad "+ofToString(i), quads[i].isOn);
    }

    // then two pages of settings for each instantiable layer
    for(int i = 0; i < 20; i++)
    {
    gui.addPage("quad "+ofToString(i)+" - 1/2");
    gui.addTitle("quad n. "+ofToString(i));
    gui.addToggle("show/hide", quads[i].isOn);
    gui.addToggle("img bg on/off", quads[i].imgBg);
    gui.addComboBox("image bg", quads[i].bgImg, imgFiles.size(), images);
    gui.addSlider("img mult X", quads[i].imgMultX, 0.2, 4.0);
    gui.addSlider("img mult Y", quads[i].imgMultY, 0.2, 4.0);
    gui.addColorPicker("img colorize", &quads[i].imgColorize.r);
    gui.addTitle("Solid color").setNewColumn(true);
    gui.addToggle("solid bg on/off", quads[i].colorBg);
    gui.addColorPicker("Color", &quads[i].bgColor.r);
    gui.addToggle("transition color", quads[i].transBg);
    gui.addColorPicker("second Color", &quads[i].secondColor.r);
    gui.addSlider("trans duration", quads[i].transDuration, 0.2, 60.0);

    gui.addPage("quad "+ofToString(i)+" - 2/2");
    gui.addTitle("Video");
    gui.addToggle("video bg on/off", quads[i].videoBg);
    gui.addComboBox("video bg", quads[i].bgVideo, videoFiles.size(), videos);
    gui.addSlider("video mult X", quads[i].videoMultX, 0.2, 4.0);
    gui.addSlider("video mult Y", quads[i].videoMultY, 0.2, 4.0);
    gui.addColorPicker("video colorize", &quads[i].videoColorize.r);
    gui.addSlider("video sound vol", quads[i].videoVolume, 0, 100);
    gui.addSlider("video speed", quads[i].videoSpeed, -2.0, 4.0);
    gui.addTitle("Camera bg").setNewColumn(true);
    gui.addToggle("cam on/off", quads[i].camBg);
    gui.addSlider("camera mult X", quads[i].camMultX, 0.2, 4.0);
    gui.addSlider("camera mult Y", quads[i].camMultY, 0.2, 4.0);
    gui.addColorPicker("cam colorize", &quads[i].camColorize.r);
    gui.addTitle("Slideshow").setNewColumn(true);
    gui.addToggle("slideshow on/off", quads[i].slideshowBg);
    gui.addComboBox("slideshow folder", quads[i].bgSlideshow, slideshowFolders.size(), slideshows);
    gui.addSlider("slide duration", quads[i].slideshowSpeed, 0.1, 15.0);
    gui.addToggle("slides to quad size", quads[i].slideFit);
    gui.addToggle("keep aspect ratio", quads[i].slideKeepAspect);
    }

    // then we set displayed gui page to the one corresponding to active quad and show the gui
    gui.setPage((activeQuad*2)+2);
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
		for (int i = 0; i < 20; i++) {
			if (quads[i].initialized) {
				if (quads[i].camBg) {
				quads[i].camTexture.loadData(pixels, camWidth,camHeight, GL_RGB);
				quads[i].camWidth = camWidth;
				quads[i].camHeight = camHeight;
				}
			}
		}
	}


    // sets default window background, grey in setup mode and black in projection mode
    if (isSetup) {
    ofBackground(20, 20, 20);
    }
    else {
    ofBackground(0, 0, 0);
    }
    //ofSetWindowShape(800, 600);
    // loops through initialized quads and runs update, setting the border color as well
    for(int i = 0; i < 20; i++)
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
        // if snapshot is on draws it as window background
        if (snapshotOn) {
        ofEnableAlphaBlending();
	    ofSetColor(0xFFFFFF);
	    snapshotTexture.draw(0,0,ofGetWidth(),ofGetHeight());
	    ofDisableAlphaBlending();
        }
    }

    // loops through initialized quads and calls their draw function
    for(int i = 0; i < 20; i++)
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
    // saves quads settings to an xml file in data directory
    if ( key == 's' || key == 'S')
    {
        setXml();
        XML.saveFile("projection_settings.xml");

    }

    // loads settings and quads from default xml file
    if (key == 'l' || key == 'L')
    {
    XML.loadFile("projection_settings.xml");
    getXml();
    }

    // takes a snapshot of attached camera and uses it as window background
    if (key == 'w' || key == 'W')
    {
    snapshotOn = !snapshotOn;
    if (snapshotOn == 1) {
    camGrabber.grabFrame();
    int totalPixels = camWidth*camHeight*3;
    unsigned char * pixels = camGrabber.getPixels();
    snapshotTexture.loadData(pixels, camWidth,camHeight, GL_RGB);
    }
    }

    // fills window with active quad
    if ( key =='q' || key == 'Q' )
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
    gui.setPage((activeQuad*2)+2);
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
    gui.setPage((activeQuad*2)+2);
    }

    // goes to first page of gui for active quad
    if ( key == 'z' || key == 'Z')
    {
        gui.setPage((activeQuad*2)+2);
    }

    // goes to second page of gui for active quad
        if ( key == 'x' || key == 'X')
    {
        gui.setPage((activeQuad*2)+3);
    }

    // adds a new quad in the middle of the screen
    if ( key =='a' )
    {
        if (isSetup)
        {
            if (nOfQuads < 20)
            {
                quads[nOfQuads].setup(0.25,0.25,0.75,0.25,0.75,0.75,0.25,0.75, imgFiles, videoFiles, slideshowFolders);
                quads[nOfQuads].quadNumber = nOfQuads;
                activeQuad = nOfQuads;
                ++nOfQuads;
                gui.setPage((activeQuad*2)+2);
            }
        }
    }

    // toggles setup mode
    if ( key ==' ' )
    {
        if (isSetup)
        {
            isSetup = False;
            for(int i = 0; i < 20; i++)
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
            for(int i = 0; i < 20; i++)
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
    bGui = !bGui;
    }

    if(key == '[')
    {
    gui.prevPage();
    }

    if(key == ']')
    {
    gui.nextPage();
    }

    // show general settings page of gui
    if(key == '1')
    {
    gui.setPage(1);
    }

    // resyncs videos to start point in every quad
    if(key == 'r' || key == 'R')
    {
    for(int i = 0; i < 20; i++)
            {
                if (quads[i].initialized)
                {
                    // resets video to start ing point
                    if (quads[i].videoBg && quads[i].video.bLoaded) {
                    quads[i].video.setPosition(0.0);
                    }
                    // resets slideshow to first slide
                    if (quads[i].slideshowBg) {
                        quads[i].currentSlide = 0;
                        quads[i].slideTimer = ofGetElapsedTimef();
                    }
                }
            }
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
    if (isSetup && !bGui)
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
    if (isSetup && !bGui)
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
    if (whichCorner >= 0) {
        // snap detection for near quads
        float smallestDist = 1.0;
        int snapQuad = -1;
        int snapCorner = -1;
        for (int i = 0; i < 20; i++) {
            if ( i != activeQuad && quads[i].initialized) {
                for(int j = 0; j < 4; j++) {
                    float distx = quads[activeQuad].corners[whichCorner].x - quads[i].corners[j].x;
                    float disty = quads[activeQuad].corners[whichCorner].y - quads[i].corners[j].y;
                    float dist = sqrt( distx * distx + disty * disty);
                    if (dist < smallestDist && dist < 0.0075) {
                        snapQuad = i;
                        snapCorner = j;
                        smallestDist = dist;
                    }
                }
            }
        }
        if (snapQuad >= 0 && snapCorner >= 0) {
            quads[activeQuad].corners[whichCorner].x = quads[snapQuad].corners[snapCorner].x;
            quads[activeQuad].corners[whichCorner].y = quads[snapQuad].corners[snapCorner].y;
    }
    }
    whichCorner = -1;
}



//--------------------------------------------------------------

void testApp::setXml()

{
XML.setValue("GENERAL:ACTIVE_QUAD",activeQuad);
XML.setValue("GENERAL:N_OF_QUADS",nOfQuads);

for(int i = 0; i < 20; i++)
    {
     if (quads[i].initialized)
     {

        XML.setValue("QUADS:QUAD_"+ofToString(i)+":QUAD_NUMBER",quads[i].quadNumber);

        XML.setValue("QUADS:QUAD_"+ofToString(i)+":LOADED_IMG",quads[i].bgImg);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":LOADED_VIDEO",quads[i].bgVideo);


        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_0:X",quads[i].corners[0].x);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_0:Y",quads[i].corners[0].y);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_1:X",quads[i].corners[1].x);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_1:Y",quads[i].corners[1].y);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_2:X",quads[i].corners[2].x);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_2:Y",quads[i].corners[2].y);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_3:X",quads[i].corners[3].x);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_3:Y",quads[i].corners[3].y);

        XML.setValue("QUADS:QUAD_"+ofToString(i)+":IS_ON",quads[i].isOn);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR_BG",quads[i].colorBg);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM_BG",quads[i].camBg);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG_BG",quads[i].imgBg);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_BG",quads[i].videoBg);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM_WIDTH",quads[i].camWidth);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM_HEIGHT",quads[i].camHeight);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM_MULT_X",quads[i].camMultX);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM_MULT_Y",quads[i].camMultY);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG_MULT_X",quads[i].imgMultX);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG_MULT_Y",quads[i].imgMultY);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_MULT_X",quads[i].videoMultX);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_MULT_Y",quads[i].videoMultY);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_SPEED",quads[i].videoSpeed);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_VOLUME",quads[i].videoVolume);

        XML.setValue("QUADS:QUAD_"+ofToString(i)+":BG_COLOR:R",quads[i].bgColor.r);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":BG_COLOR:G",quads[i].bgColor.g);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":BG_COLOR:B",quads[i].bgColor.b);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":BG_COLOR:A",quads[i].bgColor.a);

        XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG_COLORIZE:R",quads[i].imgColorize.r);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG_COLORIZE:G",quads[i].imgColorize.g);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG_COLORIZE:B",quads[i].imgColorize.b);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG_COLORIZE:A",quads[i].imgColorize.a);

        XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_COLORIZE:R",quads[i].videoColorize.r);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_COLORIZE:G",quads[i].videoColorize.g);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_COLORIZE:B",quads[i].videoColorize.b);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_COLORIZE:A",quads[i].videoColorize.a);

        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM_COLORIZE:R",quads[i].camColorize.r);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM_COLORIZE:G",quads[i].camColorize.g);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM_COLORIZE:B",quads[i].camColorize.b);
        XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM_COLORIZE:A",quads[i].camColorize.a);
}
}
}


void testApp::getXml()

{

nOfQuads = XML.getValue("GENERAL:N_OF_QUADS", 0);
activeQuad = XML.getValue("GENERAL:ACTIVE_QUAD", 0);

for(int i = 0; i < nOfQuads; i++)
    {
    float x0 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_0:X",0.0);
    float y0 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_0:Y",0.0);
    float x1 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_1:X",0.0);
    float y1 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_1:Y",0.0);
    float x2 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_2:X",0.0);
    float y2 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_2:Y",0.0);
    float x3 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_3:X",0.0);
    float y3 = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_3:Y",0.0);

    quads[i].setup(x0, y0, x1, y1, x2, y2, x3, y3, imgFiles, videoFiles, slideshowFolders);
    quads[i].quadNumber = XML.getValue("QUADS:QUAD_"+ofToString(i)+":QUAD_NUMBER", 0);
    quads[i].bgImg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":LOADED_IMG", 0);
    quads[i].bgVideo = XML.getValue("QUADS:QUAD_"+ofToString(i)+":LOADED_VIDEO", 0);

    /* quads[i].corners[0].x = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_0:X",0);
    quads[i].corners[0].y = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_0:Y",0);
    quads[i].corners[1].x = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_1:X",0);
    quads[i].corners[1].y = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_1:Y",0);
    quads[i].corners[2].x = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_2:X",0);
    quads[i].corners[2].y = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_2:Y",0);
    quads[i].corners[3].x = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_3:X",0);
    quads[i].corners[3].y = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_3:Y",0);
    */

    quads[i].isOn = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IS_ON",0);
    quads[i].colorBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR_BG",0);
    quads[i].camBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM_BG",0);
    quads[i].imgBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG_BG",0);
    quads[i].videoBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_BG",0);
    quads[i].camWidth = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM_WIDTH",0);
    quads[i].camHeight = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM_HEIGHT",0);
    quads[i].camMultX = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM_MULT_X",1.0);
    quads[i].camMultY = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM_MULT_Y",1.0);
    quads[i].imgMultX = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG_MULT_X",1.0);
    quads[i].imgMultY = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG_MULT_Y",1.0);
    quads[i].videoMultX = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_MULT_X",1.0);
    quads[i].videoMultY = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_MULT_Y",1.0);
    quads[i].videoSpeed = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_SPEED",1.0);
    quads[i].videoVolume = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_VOLUME",0);

    quads[i].bgColor.r = XML.getValue("QUADS:QUAD_"+ofToString(i)+":BG_COLOR:R",0.0);
    quads[i].bgColor.g = XML.getValue("QUADS:QUAD_"+ofToString(i)+":BG_COLOR:G",0.0);
    quads[i].bgColor.b = XML.getValue("QUADS:QUAD_"+ofToString(i)+":BG_COLOR:B",0.0);
    quads[i].bgColor.a = XML.getValue("QUADS:QUAD_"+ofToString(i)+":BG_COLOR:A",0.0);

    quads[i].imgColorize.r = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG_COLORIZE:R",1.0);
    quads[i].imgColorize.g = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG_COLORIZE:G",1.0);
    quads[i].imgColorize.b = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG_COLORIZE:B",1.0);
    quads[i].imgColorize.a = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG_COLORIZE:A",1.0);

    quads[i].videoColorize.r = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_COLORIZE:R",1.0);
    quads[i].videoColorize.g = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_COLORIZE:G",1.0);
    quads[i].videoColorize.b = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_COLORIZE:B",1.0);
    quads[i].videoColorize.a = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO_COLORIZE:A",1.0);

    quads[i].camColorize.r = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM_COLORIZE:R",1.0);
    quads[i].camColorize.g = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM_COLORIZE:G",1.0);
    quads[i].camColorize.b = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM_COLORIZE:B",1.0);
    quads[i].camColorize.a = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM_COLORIZE:A",1.0);

}
}
