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
    if((dp  = opendir(dir.c_str())) == NULL)
    {
        cout << "Error(" << errno << ") opening " << dir << endl;
        return errno;
    }

    while ((dirp = readdir(dp)) != NULL)
    {
        if (string(dirp->d_name) != "." && string(dirp->d_name) != "..")
        {
            files.push_back(string(dirp->d_name));
        }
    }
    closedir(dp);
    return 0;
}



//--------------------------------------------------------------
void testApp::setup()
{

    //we run at 60 fps!
    ofSetVerticalSync(true);

    // splash image
    bSplash = true;
    splashImg.loadImage("lpmt_splash.png");
    splashTime = ofGetElapsedTimef();

    // OSC setup
    receiver.setup( PORT );
    current_msg_string = 0;

    // we scan the video dir for videos
    //string videoDir = string("./data/video");
    string videoDir =  ofToDataPath("video",true);
    videoFiles = vector<string>();
    getdir(videoDir,videoFiles);
    string videos[videoFiles.size()];
    for (unsigned int i = 0; i < videoFiles.size(); i++)
    {
        videos[i]= videoFiles[i];
    }

    // we scan the slideshow dir for videos
    //string slideshowDir = string("./data/slideshow");
    string slideshowDir = ofToDataPath("slideshow",true);
    slideshowFolders = vector<string>();
    getdir(slideshowDir,slideshowFolders);
    string slideshows[slideshowFolders.size()];
    for (unsigned int i = 0; i < slideshowFolders.size(); i++)
    {
        slideshows[i]= slideshowFolders[i];
    }


    ttf.loadFont("type/frabk.ttf", 11);
    // set border color for quads in setup mode
    borderColor = 0x666666;
    // starts in quads setup mode
    isSetup = True;
    // starts running
    bStarted = True;
    // default is not using MostPixelsEver
    bMpe = False;
    // starts in windowed mode
    bFullscreen	= 0;
    // gui is on at start
    bGui = 1;
    ofSetWindowShape(WINDOW_W, WINDOW_H);

    // camera stuff
    camWidth = 640;	// try to grab at this size.
    camHeight = 480;
    camGrabber.setVerbose(true);
    camGrabber.initGrabber(camWidth,camHeight);

    // texture for snapshot background
    snapshotTexture.allocate(camWidth,camHeight, GL_RGB);
    snapshotOn = 0;

    // initializes layers array
    for(int i = 0; i < 36; i++)
    {
        layers[i] = -1;
    }


    // defines the first 4 default quads
    quads[0].setup(0.0,0.0,0.5,0.0,0.5,0.5,0.0,0.5, videoFiles, slideshowFolders);
    quads[0].quadNumber = 0;
    quads[1].setup(0.5,0.0,1.0,0.0,1.0,0.5,0.5,0.5, videoFiles, slideshowFolders);
    quads[1].quadNumber = 1;
    quads[2].setup(0.0,0.5,0.5,0.5,0.5,1.0,0.0,1.0, videoFiles, slideshowFolders);
    quads[2].quadNumber = 2;
    quads[3].setup(0.5,0.5,1.0,0.5,1.0,1.0,0.5,1.0, videoFiles, slideshowFolders);
    quads[3].quadNumber = 3;
    // define last one as active quad
    activeQuad = 3;
    // number of total quads, to be modified later at each quad insertion
    nOfQuads = 4;
    layers[0] = 0;
    quads[0].layer = 0;
    layers[1] = 1;
    quads[1].layer = 1;
    layers[2] = 2;
    quads[2].layer = 2;
    layers[3] = 3;
    quads[3].layer = 3;



    // GUI STUFF ---------------------------------------------------

    // general page
    gui.addTitle("show/hide quads");
    // overriding default theme
    //gui.bDrawHeader = false;
    gui.config->toggleHeight = 18;
    gui.config->buttonHeight = 18;
    gui.config->sliderTextHeight = 22;
    gui.config->titleHeight = 18;
    //gui.config->fullActiveColor = 0x6B404B;
    gui.config->fullActiveColor = 0x5E4D3E;
    // adding controls
    // first a general page for toggling layers on/off
    for(int i = 0; i < 36; i++)
    {
        gui.addToggle("quad "+ofToString(i), quads[i].isOn);
    }

    // then two pages of settings for each instantiable layer
    for(int i = 0; i < 36; i++)
    {
        gui.addPage("quad "+ofToString(i)+" - 1/3");
        gui.addTitle("quad n. "+ofToString(i));
        gui.addToggle("show/hide", quads[i].isOn);
        gui.addToggle("img bg on/off", quads[i].imgBg);
        gui.addButton("image bg", bImageLoad);
        gui.addSlider("img mult X", quads[i].imgMultX, 0.2, 4.0);
        gui.addSlider("img mult Y", quads[i].imgMultY, 0.2, 4.0);
        gui.addColorPicker("img colorize", &quads[i].imgColorize.r);
        gui.addTitle("Solid color").setNewColumn(true);
        gui.addToggle("solid bg on/off", quads[i].colorBg);
        gui.addColorPicker("Color", &quads[i].bgColor.r);
        gui.addToggle("transition color", quads[i].transBg);
        gui.addColorPicker("second Color", &quads[i].secondColor.r);
        gui.addSlider("trans duration", quads[i].transDuration, 0.2, 60.0);

        gui.addPage("quad "+ofToString(i)+" - 2/3");
        gui.addTitle("Video");
        gui.addToggle("video bg on/off", quads[i].videoBg);
        gui.addComboBox("video bg", quads[i].bgVideo, videoFiles.size(), videos);
        gui.addSlider("video mult X", quads[i].videoMultX, 0.2, 4.0);
        gui.addSlider("video mult Y", quads[i].videoMultY, 0.2, 4.0);
        gui.addColorPicker("video colorize", &quads[i].videoColorize.r);
        gui.addSlider("video sound vol", quads[i].videoVolume, 0, 100);
        gui.addSlider("video speed", quads[i].videoSpeed, -2.0, 4.0);
        gui.addToggle("video loop", quads[i].videoLoop);
        gui.addTitle("Camera bg").setNewColumn(true);
        gui.addToggle("cam on/off", quads[i].camBg);
        gui.addSlider("camera mult X", quads[i].camMultX, 0.2, 4.0);
        gui.addSlider("camera mult Y", quads[i].camMultY, 0.2, 4.0);
        gui.addColorPicker("cam colorize", &quads[i].camColorize.r);
        gui.addTitle("Greenscreen");
        gui.addSlider("g-screen threshold", quads[i].thresholdGreenscreen, 0, 120);
        gui.addColorPicker("greenscreen col", &quads[i].colorGreenscreen.r);
        gui.addToggle("video greenscreen", quads[i].videoGreenscreen);
        gui.addToggle("camera greenscreen", quads[i].camGreenscreen);
        gui.addTitle("Slideshow").setNewColumn(true);
        gui.addToggle("slideshow on/off", quads[i].slideshowBg);
        gui.addComboBox("slideshow folder", quads[i].bgSlideshow, slideshowFolders.size(), slideshows);
        gui.addSlider("slide duration", quads[i].slideshowSpeed, 0.1, 15.0);
        gui.addToggle("slides to quad size", quads[i].slideFit);
        gui.addToggle("keep aspect ratio", quads[i].slideKeepAspect);

        gui.addPage("quad "+ofToString(i)+" - 3/3");
        gui.addTitle("Corner 0");
        gui.addSlider("X", quads[i].corners[0].x, -1.0, 2.0);
        gui.addSlider("Y", quads[i].corners[0].y, -1.0, 2.0);
        gui.addTitle("Corner 3");
        gui.addSlider("X", quads[i].corners[3].x, -1.0, 2.0);
        gui.addSlider("Y", quads[i].corners[3].y, -1.0, 2.0);
        gui.addTitle("Corner 1").setNewColumn(true);
        gui.addSlider("X", quads[i].corners[1].x, -1.0, 2.0);
        gui.addSlider("Y", quads[i].corners[1].y, -1.0, 2.0);
        gui.addTitle("Corner 2");
        gui.addSlider("X", quads[i].corners[2].x, -1.0, 2.0);
        gui.addSlider("Y", quads[i].corners[2].y, -1.0, 2.0);
    }

    // then we set displayed gui page to the one corresponding to active quad and show the gui
    gui.setPage((activeQuad*3)+2);
    gui.show();

}


void testApp::openImageFile()
{
    ofFileDialogResult dialog_result = ofSystemLoadDialog("load image", false);
    if(dialog_result.bSuccess)
    {
        quads[activeQuad].loadImageFromFile(dialog_result.getName(), dialog_result.getPath());
    }
}


void testApp::mpeSetup()
{
    stopProjection();
    bMpe = True;
    // MPE stuff
    lastFrameTime = ofGetElapsedTimef();
    client.setup("mpe_client_settings.xml", true); //false means you can use backthread
    ofxMPERegisterEvents(this);
    //resync();
    startProjection();
    client.start();
    ofSetBackgroundAuto(false);
}


//--------------------------------------------------------------
void testApp::prepare()
{
    // check for waiting OSC messages
    while( receiver.hasWaitingMessages() )
    {
        parseOsc();
    }

    if (bStarted)
    {
        // check if image load button on GUI is pressed
        if(bImageLoad)
        {
            bImageLoad = false;
            openImageFile();
        }

        // grabs video frame from camera and passes pixels to quads

        if (camGrabber.getHeight() > 0)  // isLoaded check
        {
            camGrabber.grabFrame();
            if (camGrabber.isFrameNew())
            {
                int totalPixels = camWidth*camHeight*3;
                unsigned char * pixels = camGrabber.getPixels();
                for (int j = 0; j < 36; j++)
                {
                    int i = layers[j];
                    if (quads[i].initialized)
                    {
                        if (quads[i].camBg)
                        {
                            quads[i].camPixels = pixels;
                            quads[i].camWidth = camWidth;
                            quads[i].camHeight = camHeight;
                        }
                    }
                }
            }
        }


        // sets default window background, grey in setup mode and black in projection mode
        if (isSetup)
        {
            ofBackground(20, 20, 20);
        }
        else
        {
            ofBackground(0, 0, 0);
        }
        //ofSetWindowShape(800, 600);
        // loops through initialized quads and runs update, setting the border color as well
        for(int j = 0; j < 36; j++)
        {
            int i = layers[j];
            if (quads[i].initialized)
            {
                quads[i].update();
                quads[i].borderColor = borderColor;
            }
        }

    }
}


//--------------------------------------------------------------
void testApp::dostuff()
{
    if (bStarted)
    {
        // loops through initialized quads and calls their draw function
        for(int j = 0; j < 36; j++)
        {
            int i = layers[j];
            if (quads[i].initialized)
            {
                quads[i].draw();
            }
        }

    }
}

//--------------------------------------------------------------
void testApp::update()
{

    if (!bMpe)
    {
        if (bSplash)
        {
            if (abs(splashTime - ofGetElapsedTimef()) > 8.0)
            {
                bSplash = ! bSplash;
            }
        }
        prepare();
    }
}

//--------------------------------------------------------------
void testApp::draw()
{
    // in setup mode writes the number of active quad at the bottom of the window
    if (isSetup)
    {
        // in setup mode sets active quad border to be white
        quads[activeQuad].borderColor = 0xFFFFFF;
    }

    if (!bMpe)
    {
        dostuff();
    }

    if (isSetup)
    {

        if (bStarted)
        {
            // if snapshot is on draws it as window background
            if (snapshotOn)
            {
                ofEnableAlphaBlending();
                ofSetHexColor(0xFFFFFF);
                snapshotTexture.draw(0,0,ofGetWidth(),ofGetHeight());
                ofDisableAlphaBlending();
            }
            ofSetHexColor(0xFFFFFF);
            ttf.drawString("active quad: "+ofToString(activeQuad), 30, ofGetHeight()-25);
            // draws gui
            gui.draw();
        }
    }

    if (bSplash)
    {
        ofEnableAlphaBlending();
        splashImg.draw(((ofGetWidth()/2)-165),((ofGetHeight()/2)-125));
        ofDisableAlphaBlending();
    }
}


//--------------------------------------------------------------
void testApp::parseOsc()
{
    // get the next message
    ofxOscMessage m;
    receiver.getNextMessage( &m );

    // check for quads corner x movements
    if ( m.getAddress() == "/corners/x" )
    {
        // arguments are iif
        int osc_quad = m.getArgAsInt32( 0 );
        int osc_corner = m.getArgAsInt32( 1 );
        float osc_coord = m.getArgAsFloat( 2 );
        quads[osc_quad].corners[osc_corner].x = osc_coord;
    }
    // check for quads corner y movements
    else if ( m.getAddress() == "/corners/y" )
    {
        // arguments are iif
        int osc_quad = m.getArgAsInt32( 0 );
        int osc_corner = m.getArgAsInt32( 1 );
        float osc_coord = m.getArgAsFloat( 2 );
        quads[osc_quad].corners[osc_corner].y = osc_coord;
    }

    // check for active quad corner x movements
    if ( m.getAddress() == "/corners/active/0" )
    {
        // arguments are ff
        float osc_coord_x = m.getArgAsFloat( 0 );
	float osc_coord_y = m.getArgAsFloat( 1 );
        quads[activeQuad].corners[0].x = osc_coord_x;
	quads[activeQuad].corners[0].y = osc_coord_y;
    }

    if ( m.getAddress() == "/corners/active/1" )
    {
        // arguments are ff
        float osc_coord_x = m.getArgAsFloat( 0 );
	float osc_coord_y = m.getArgAsFloat( 1 );
        quads[activeQuad].corners[1].x = osc_coord_x;
	quads[activeQuad].corners[1].y = osc_coord_y;
    }

    if ( m.getAddress() == "/corners/active/2" )
    {
        // arguments are ff
        float osc_coord_x = m.getArgAsFloat( 0 );
	float osc_coord_y = m.getArgAsFloat( 1 );
        quads[activeQuad].corners[2].x = osc_coord_x;
	quads[activeQuad].corners[2].y = osc_coord_y;
    }

    if ( m.getAddress() == "/corners/active/3" )
    {
        // arguments are ff
        float osc_coord_x = m.getArgAsFloat( 0 );
	float osc_coord_y = m.getArgAsFloat( 1 );
        quads[activeQuad].corners[3].x = osc_coord_x;
	quads[activeQuad].corners[3].y = osc_coord_y;
    }

    // change active quad
    else if ( m.getAddress() == "/active" )
    {
        // argument is int32
        int osc_activequad = m.getArgAsInt32( 0 );
        if (osc_activequad <= nOfQuads-1)
        {
            activeQuad = osc_activequad;
            gui.setPage((activeQuad*3)+2);
        }
    }

    // resync
    else if ( m.getAddress() == "/resync" )
    {
        resync();
    }

    // stop
    else if ( m.getAddress() == "/stop" )
    {
        stopProjection();
    }

    // start
    else if ( m.getAddress() == "/start" )
    {
        startProjection();
    }

    // toggle fullscreen
    else if ( m.getAddress() == "/fullscreen" )
    {
        bFullscreen = !bFullscreen;

        if(!bFullscreen)
        {
            ofSetWindowShape(WINDOW_W, WINDOW_H);
            ofSetFullscreen(false);
            // figure out how to put the window in the center:
            int screenW = ofGetScreenWidth();
            int screenH = ofGetScreenHeight();
            ofSetWindowPosition(screenW/2-WINDOW_W/2, screenH/2-WINDOW_H/2);
        }
        else if(bFullscreen == 1)
        {
            ofSetFullscreen(true);
        }
    }

    // toggle gui
    else if ( m.getAddress() == "/gui" )
    {
        gui.toggleDraw();
        bGui = !bGui;
    }

    // toggle setup
    else if ( m.getAddress() == "/setup" )
    {
        if (isSetup)
        {
            isSetup = False;
            for(int i = 0; i < 36; i++)
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
            for(int i = 0; i < 36; i++)
            {
                if (quads[i].initialized)
                {
                    quads[i].isSetup = True;
                }
            }
        }
    }

    // connects to mpe server
    else if ( m.getAddress() == "/mpe" )
    {
        mpeSetup();
    }

    // img stuff on active quad
    else if ( m.getAddress() == "/img" )
    {
        quads[activeQuad].imgBg = !quads[activeQuad].imgBg;
    }

    else if ( m.getAddress() == "/img/color" )
    {
        // arguments are ffff
        float img_color_r = m.getArgAsFloat( 0 );
        float img_color_g = m.getArgAsFloat( 1 );
        float img_color_b = m.getArgAsFloat( 2 );
        float img_color_a = m.getArgAsFloat( 3 );
        quads[activeQuad].imgColorize.r = img_color_r;
        quads[activeQuad].imgColorize.g = img_color_g;
        quads[activeQuad].imgColorize.b = img_color_b;
        quads[activeQuad].imgColorize.a = img_color_a;
    }

    else if ( m.getAddress() == "/img/color/1" )
    {
        // arguments are f
        float img_color_r = m.getArgAsFloat( 0 );
        quads[activeQuad].imgColorize.r = img_color_r;
    }

    else if ( m.getAddress() == "/img/color/2" )
    {
        // arguments are f
        float img_color_g = m.getArgAsFloat( 0 );
        quads[activeQuad].imgColorize.g = img_color_g;
    }

    else if ( m.getAddress() == "/img/color/3" )
    {
        // arguments are f
        float img_color_b = m.getArgAsFloat( 0 );
        quads[activeQuad].imgColorize.b = img_color_b;
    }

    else if ( m.getAddress() == "/img/color/4" )
    {
        // arguments are f
        float img_color_a = m.getArgAsFloat( 0 );
        quads[activeQuad].imgColorize.a = img_color_a;
    }

    else if ( m.getAddress() == "/img/mult/x" )
    {
        // arguments are f
        float img_mult_x = m.getArgAsFloat( 0 );
        quads[activeQuad].imgMultX = img_mult_x;
    }

    else if ( m.getAddress() == "/img/mult/y" )
    {
        // arguments are f
        float img_mult_y = m.getArgAsFloat( 0 );
        quads[activeQuad].imgMultY = img_mult_y;
    }

    // solid color stuff
    else if ( m.getAddress() == "/solid" )
    {
        quads[activeQuad].colorBg = !quads[activeQuad].colorBg;
    }

    else if ( m.getAddress() == "/solid/color" )
    {
        // arguments are ffff
        float solid_color_r = m.getArgAsFloat( 0 );
        float solid_color_g = m.getArgAsFloat( 1 );
        float solid_color_b = m.getArgAsFloat( 2 );
        float solid_color_a = m.getArgAsFloat( 3 );
        quads[activeQuad].bgColor.r = solid_color_r;
        quads[activeQuad].bgColor.g = solid_color_g;
        quads[activeQuad].bgColor.b = solid_color_b;
        quads[activeQuad].bgColor.a = solid_color_a;
    }

    else if ( m.getAddress() == "/solid/color/1" )
    {
        // arguments are f
        float solid_color_r = m.getArgAsFloat( 0 );
        quads[activeQuad].bgColor.r = solid_color_r;
    }

    else if ( m.getAddress() == "/solid/color/2" )
    {
        // arguments are f
        float solid_color_g = m.getArgAsFloat( 0 );
        quads[activeQuad].bgColor.g = solid_color_g;
    }

    else if ( m.getAddress() == "/solid/color/3" )
    {
        // arguments are f
        float solid_color_b = m.getArgAsFloat( 0 );
        quads[activeQuad].bgColor.b = solid_color_b;
    }

    else if ( m.getAddress() == "/solid/color/4" )
    {
        // arguments are f
        float solid_color_a = m.getArgAsFloat( 0 );
        quads[activeQuad].bgColor.a = solid_color_a;
    }

    else if ( m.getAddress() == "/solid/trans" )
    {
        quads[activeQuad].transBg = !quads[activeQuad].transBg;
    }

    else if ( m.getAddress() == "/solid/trans/color" )
    {
        // arguments are ffff
        float trans_color_r = m.getArgAsFloat( 0 );
        float trans_color_g = m.getArgAsFloat( 1 );
        float trans_color_b = m.getArgAsFloat( 2 );
        float trans_color_a = m.getArgAsFloat( 3 );
        quads[activeQuad].secondColor.r = trans_color_r;
        quads[activeQuad].secondColor.g = trans_color_g;
        quads[activeQuad].secondColor.b = trans_color_b;
        quads[activeQuad].secondColor.a = trans_color_a;
    }

    else if ( m.getAddress() == "/solid/trans/color/1" )
    {
        // arguments are f
        float trans_color_r = m.getArgAsFloat( 0 );
        quads[activeQuad].secondColor.r = trans_color_r;
    }

    else if ( m.getAddress() == "/solid/trans/color/2" )
    {
        // arguments are f
        float trans_color_g = m.getArgAsFloat( 0 );
        quads[activeQuad].secondColor.g = trans_color_g;
    }

    else if ( m.getAddress() == "/solid/trans/color/3" )
    {
        // arguments are f
        float trans_color_b = m.getArgAsFloat( 0 );
        quads[activeQuad].secondColor.b = trans_color_b;
    }

    else if ( m.getAddress() == "/solid/trans/color/4" )
    {
        // arguments are f
        float trans_color_a = m.getArgAsFloat( 0 );
        quads[activeQuad].secondColor.a = trans_color_a;
    }

    else if ( m.getAddress() == "/solid/trans/duration" )
    {
        // arguments are f
        float trans_duration = m.getArgAsFloat( 0 );
        quads[activeQuad].transDuration = trans_duration;
    }

    // img stuff on active quad
    else if ( m.getAddress() == "/video" )
    {
        quads[activeQuad].videoBg = !quads[activeQuad].videoBg;
    }

    else if ( m.getAddress() == "/video/color" )
    {
        // arguments are ffff
        float video_color_r = m.getArgAsFloat( 0 );
        float video_color_g = m.getArgAsFloat( 1 );
        float video_color_b = m.getArgAsFloat( 2 );
        float video_color_a = m.getArgAsFloat( 3 );
        quads[activeQuad].videoColorize.r = video_color_r;
        quads[activeQuad].videoColorize.g = video_color_g;
        quads[activeQuad].videoColorize.b = video_color_b;
        quads[activeQuad].videoColorize.a = video_color_a;
    }

    else if ( m.getAddress() == "/video/color/1" )
    {
        // arguments are f
        float video_color_r = m.getArgAsFloat( 0 );
        quads[activeQuad].videoColorize.r = video_color_r;
    }

    else if ( m.getAddress() == "/video/color/2" )
    {
        // arguments are f
        float video_color_g = m.getArgAsFloat( 0 );
        quads[activeQuad].videoColorize.g = video_color_g;
    }

    else if ( m.getAddress() == "/video/color/3" )
    {
        // arguments are f
        float video_color_b = m.getArgAsFloat( 0 );
        quads[activeQuad].videoColorize.b = video_color_b;
    }

    else if ( m.getAddress() == "/video/color/4" )
    {
        // arguments are f
        float video_color_a = m.getArgAsFloat( 0 );
        quads[activeQuad].videoColorize.a = video_color_a;
    }

    else if ( m.getAddress() == "/video/mult/x" )
    {
        // arguments are f
        float video_mult_x = m.getArgAsFloat( 0 );
        quads[activeQuad].videoMultX = video_mult_x;
    }

    else if ( m.getAddress() == "/video/mult/y" )
    {
        // arguments are f
        float video_mult_y = m.getArgAsFloat( 0 );
        quads[activeQuad].videoMultY = video_mult_y;
    }

    else if ( m.getAddress() == "/video/speed" )
    {
        // arguments are f
        float video_speed = m.getArgAsFloat( 0 );
        quads[activeQuad].videoSpeed = video_speed;
    }

    else if ( m.getAddress() == "/video/volume" )
    {
        // arguments are i
        int video_volume = m.getArgAsInt32( 0 );
        quads[activeQuad].videoVolume = video_volume;
    }


    else
    {
        // unrecognized message: display on the bottom of the screen
        string msg_string;
        msg_string = m.getAddress();
        msg_string += ": ";
        for ( int i=0; i<m.getNumArgs(); i++ )
        {
            // get the argument type
            msg_string += m.getArgTypeName( i );
            msg_string += ":";
            // display the argument - make sure we get the right type
            if( m.getArgType( i ) == OFXOSC_TYPE_INT32 )
                msg_string += ofToString( m.getArgAsInt32( i ) );
            else if( m.getArgType( i ) == OFXOSC_TYPE_FLOAT )
                msg_string += ofToString( m.getArgAsFloat( i ) );
            else if( m.getArgType( i ) == OFXOSC_TYPE_STRING )
                msg_string += m.getArgAsString( i );
            else
                msg_string += "unknown";
        }
        // add to the list of strings to display
        msg_strings[current_msg_string] = msg_string;
        timers[current_msg_string] = ofGetElapsedTimef() + 5.0f;
        current_msg_string = ( current_msg_string + 1 ) % NUM_MSG_STRINGS;
        // clear the next line
        msg_strings[current_msg_string] = "";
    }
}



//--------------------------------------------------------------
void testApp::mpeFrameEvent(ofxMPEEventArgs& event)
{
    if (bMpe)
    {
        if(client.getFrameCount()<=1)
        {
            resync();
        }
        prepare();
        dostuff();
    }
}

//--------------------------------------------------------------
void testApp::mpeMessageEvent(ofxMPEEventArgs& event)
{
    //received a message from the server
}


void testApp::mpeResetEvent(ofxMPEEventArgs& event)
{
    //triggered if the server goes down, another client goes offline, or a reset was manually triggered in the server code
}



//--------------------------------------------------------------
void testApp::keyPressed(int key)
{

    // moves active layer one position up
    if ( key == '+')
    {
        int position;
        int target;

        for(int i = 0; i < 35; i++)
        {
            if (layers[i] == quads[activeQuad].quadNumber)
            {
                position = i;
                target = i+1;
            }

        }
        if (layers[target] != -1)
        {
            int target_content = layers[target];
            layers[target] = quads[activeQuad].quadNumber;
            layers[position] = target_content;
            quads[activeQuad].layer = target;
            quads[target_content].layer = position;
        }
    }


    // moves active layer one position down
    if ( key == '-')
    {
        int position;
        int target;

        for(int i = 0; i < 36; i++)
        {
            if (layers[i] == quads[activeQuad].quadNumber)
            {
                position = i;
                target = i-1;
            }

        }
        if (target >= 0)
        {
            if (layers[target] != -1)
            {
                int target_content = layers[target];
                layers[target] = quads[activeQuad].quadNumber;
                layers[position] = target_content;
                quads[activeQuad].layer = target;
                quads[target_content].layer = position;
            }
        }
    }


    // saves quads settings to an xml file in data directory
    if ( key == 's' || key == 'S')
    {
        setXml();
        XML.saveFile("_lpmt_settings.xml");
        cout<<"saved settings to data/_lpmt_settings.xml"<<endl;

    }

    // loads settings and quads from default xml file
    if (key == 'l' || key == 'L')
    {
        XML.loadFile("_lpmt_settings.xml");
        getXml();
        cout<<"loaded settings from data/_lpmt_settings.xml"<<endl;
        gui.setPage((activeQuad*3)+2);
    }

    // takes a snapshot of attached camera and uses it as window background
    if (key == 'w' || key == 'W')
    {
        snapshotOn = !snapshotOn;
        if (snapshotOn == 1)
        {
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
        gui.setPage((activeQuad*3)+2);
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
        gui.setPage((activeQuad*3)+2);
    }

    // goes to first page of gui for active quad
    if ( key == 'z' || key == 'Z' || key == OF_KEY_F1)
    {
        gui.setPage((activeQuad*3)+2);
    }

    // goes to second page of gui for active quad
    if ( key == 'x' || key == 'X' || key == OF_KEY_F2)
    {
        gui.setPage((activeQuad*3)+3);
    }

    // goes to second page of gui for active quad
    if ( key == 'c' || key == 'C' || key == OF_KEY_F3)
    {
        gui.setPage((activeQuad*3)+4);
    }


    // adds a new quad in the middle of the screen
    if ( key =='a' )
    {
        if (isSetup)
        {
            if (nOfQuads < 36)
            {
                quads[nOfQuads].setup(0.25,0.25,0.75,0.25,0.75,0.75,0.25,0.75, videoFiles, slideshowFolders);
                quads[nOfQuads].quadNumber = nOfQuads;
                layers[nOfQuads] = nOfQuads;
                quads[nOfQuads].layer = nOfQuads;
                activeQuad = nOfQuads;
                ++nOfQuads;
                gui.setPage((activeQuad*3)+2);
            }
        }
    }

    // toggles setup mode
    if ( key ==' ' )
    {
        if (isSetup)
        {
            isSetup = False;
            for(int i = 0; i < 36; i++)
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
            for(int i = 0; i < 36; i++)
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
            ofSetWindowShape(WINDOW_W, WINDOW_H);
            ofSetFullscreen(false);
            // figure out how to put the window in the center:
            int screenW = ofGetScreenWidth();
            int screenH = ofGetScreenHeight();
            ofSetWindowPosition(screenW/2-WINDOW_W/2, screenH/2-WINDOW_H/2);
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
        resync();
    }


    // starts and stops rendering

    if(key == 'p')
    {
        startProjection();
    }

    if(key == 'o')
    {
        stopProjection();
    }

    if(key == 'm')
    {
        mpeSetup();
    }

    // displays help in system dialog
    if(key == 'h')
    {
        ofBuffer buf = ofBufferFromFile("help_keys.txt");
        ofSystemAlertDialog(buf);
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

    if(bSplash)
    {
        bSplash = !bSplash;
    }

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
    if (whichCorner >= 0)
    {
        // snap detection for near quads
        float smallestDist = 1.0;
        int snapQuad = -1;
        int snapCorner = -1;
        for (int i = 0; i < 36; i++)
        {
            if ( i != activeQuad && quads[i].initialized)
            {
                for(int j = 0; j < 4; j++)
                {
                    float distx = quads[activeQuad].corners[whichCorner].x - quads[i].corners[j].x;
                    float disty = quads[activeQuad].corners[whichCorner].y - quads[i].corners[j].y;
                    float dist = sqrt( distx * distx + disty * disty);
                    // to tune snapping change dist value inside next if statement
                    if (dist < smallestDist && dist < 0.0075)
                    {
                        snapQuad = i;
                        snapCorner = j;
                        smallestDist = dist;
                    }
                }
            }
        }
        if (snapQuad >= 0 && snapCorner >= 0)
        {
            quads[activeQuad].corners[whichCorner].x = quads[snapQuad].corners[snapCorner].x;
            quads[activeQuad].corners[whichCorner].y = quads[snapQuad].corners[snapCorner].y;
        }
    }
    whichCorner = -1;
}


//--------------------------------------------------------------
void testApp::resync()
{
    for(int i = 0; i < 36; i++)
    {
        if (quads[i].initialized)
        {
            // resets video to start ing point
            if (quads[i].videoBg && quads[i].video.isLoaded())
            {
                quads[i].video.setPosition(0.0);
            }
            // resets slideshow to first slide
            if (quads[i].slideshowBg)
            {
                quads[i].currentSlide = 0;
                quads[i].slideTimer = 0;
            }
            // reset trans colors
            if (quads[i].colorBg && quads[i].transBg)
            {
                quads[i].transCounter = 0;
                quads[i].transUp = True;
            }
        }
    }
}

//--------------------------------------------------------------
void testApp::startProjection()
{
    bStarted = True;
    for(int i = 0; i < 36; i++)
    {
        if (quads[i].initialized)
        {
            quads[i].isOn = True;
            if (quads[i].videoBg && quads[i].video.isLoaded())
            {
                quads[i].video.setVolume(quads[i].videoVolume);
                quads[i].video.play();
            }
        }
    }
}

//--------------------------------------------------------------
void testApp::stopProjection()
{
    bStarted = False;
    for(int i = 0; i < 36; i++)
    {
        if (quads[i].initialized)
        {
            quads[i].isOn = False;
            if (quads[i].videoBg && quads[i].video.isLoaded())
            {
                quads[i].video.setVolume(0);
                quads[i].video.stop();
            }
        }
    }
}


//--------------------------------------------------------------

void testApp::setXml()

{
    XML.setValue("GENERAL:ACTIVE_QUAD",activeQuad);
    XML.setValue("GENERAL:N_OF_QUADS",nOfQuads);

    for(int i = 0; i < 36; i++)
    {
        if (quads[i].initialized)
        {

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":NUMBER",quads[i].quadNumber);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":LAYER",quads[i].layer);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:LOADED_IMG",quads[i].loadedImg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:LOADED_IMG_PATH",quads[i].bgImg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:LOADED_VIDEO",quads[i].bgVideo);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:LOADED_SLIDESHOW",quads[i].bgSlideshow);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_0:X",quads[i].corners[0].x);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_0:Y",quads[i].corners[0].y);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_1:X",quads[i].corners[1].x);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_1:Y",quads[i].corners[1].y);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_2:X",quads[i].corners[2].x);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_2:Y",quads[i].corners[2].y);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_3:X",quads[i].corners[3].x);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CORNERS:CORNER_3:Y",quads[i].corners[3].y);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IS_ON",quads[i].isOn);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:ACTIVE",quads[i].colorBg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:TRANS:ACTIVE",quads[i].transBg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:TRANS:DURATION",quads[i].transDuration);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:ACTIVE",quads[i].slideshowBg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:SPEED",quads[i].slideshowSpeed);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:FIT",quads[i].slideFit);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:KEEP_ASPECT",quads[i].slideKeepAspect);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:ACTIVE",quads[i].camBg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:ACTIVE",quads[i].imgBg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:ACTIVE",quads[i].videoBg);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:WIDTH",quads[i].camWidth);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:HEIGHT",quads[i].camHeight);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:MULT_X",quads[i].camMultX);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:MULT_Y",quads[i].camMultY);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:MULT_X",quads[i].imgMultX);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:MULT_Y",quads[i].imgMultY);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:MULT_X",quads[i].videoMultX);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:MULT_Y",quads[i].videoMultY);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:SPEED",quads[i].videoSpeed);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:VOLUME",quads[i].videoVolume);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:LOOP",quads[i].videoLoop);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:R",quads[i].bgColor.r);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:G",quads[i].bgColor.g);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:B",quads[i].bgColor.b);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:A",quads[i].bgColor.a);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:R",quads[i].secondColor.r);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:G",quads[i].secondColor.g);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:B",quads[i].secondColor.b);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:A",quads[i].secondColor.a);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:R",quads[i].imgColorize.r);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:G",quads[i].imgColorize.g);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:B",quads[i].imgColorize.b);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:A",quads[i].imgColorize.a);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:R",quads[i].videoColorize.r);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:G",quads[i].videoColorize.g);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:B",quads[i].videoColorize.b);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:A",quads[i].videoColorize.a);

            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:R",quads[i].camColorize.r);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:G",quads[i].camColorize.g);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:B",quads[i].camColorize.b);
            XML.setValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:A",quads[i].camColorize.a);
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

        quads[i].setup(x0, y0, x1, y1, x2, y2, x3, y3, videoFiles, slideshowFolders);
        quads[i].quadNumber = XML.getValue("QUADS:QUAD_"+ofToString(i)+":NUMBER", 0);
        quads[i].layer = XML.getValue("QUADS:QUAD_"+ofToString(i)+":LAYER", 0);
        layers[quads[i].layer] = quads[i].quadNumber;

        quads[i].imgBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:ACTIVE",0);
        quads[i].loadedImg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:LOADED_IMG", "", 0);
        quads[i].bgImg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:LOADED_IMG_PATH", "", 0);
        if ((quads[i].imgBg) && (quads[i].bgImg != ""))
        {
            quads[i].loadImageFromFile(quads[i].loadedImg, quads[i].bgImg);
        }
        quads[i].bgVideo = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:LOADED_VIDEO", 0);
        quads[i].bgSlideshow = XML.getValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:LOADED_SLIDESHOW", 0);

        quads[i].colorBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:ACTIVE",0);

        quads[i].transBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:TRANS:ACTIVE",0);
        quads[i].transDuration = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:TRANS:DURATION", 1.0);
        quads[i].slideshowBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:ACTIVE", 0);
        quads[i].slideshowSpeed = XML.getValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:SPEED", 1.0);
        quads[i].slideFit = XML.getValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:FIT", 0);
        quads[i].slideKeepAspect = XML.getValue("QUADS:QUAD_"+ofToString(i)+":SLIDESHOW:KEEP_ASPECT", 1);


        quads[i].camBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:ACTIVE",0);
        quads[i].videoBg = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:ACTIVE",0);
        quads[i].camWidth = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:WIDTH",0);
        quads[i].camHeight = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:HEIGHT",0);
        quads[i].camMultX = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:MULT_X",1.0);
        quads[i].camMultY = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:MULT_Y",1.0);
        quads[i].imgMultX = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:MULT_X",1.0);
        quads[i].imgMultY = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:MULT_Y",1.0);
        quads[i].videoMultX = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:MULT_X",1.0);
        quads[i].videoMultY = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:MULT_Y",1.0);
        quads[i].videoSpeed = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:SPEED",1.0);
        quads[i].videoVolume = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:VOLUME",0);
        quads[i].videoLoop = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:LOOP",1);

        quads[i].bgColor.r = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:R",0.0);
        quads[i].bgColor.g = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:G",0.0);
        quads[i].bgColor.b = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:B",0.0);
        quads[i].bgColor.a = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:A",0.0);

        quads[i].secondColor.r = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:R",0.0);
        quads[i].secondColor.g = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:G",0.0);
        quads[i].secondColor.b = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:B",0.0);
        quads[i].secondColor.a = XML.getValue("QUADS:QUAD_"+ofToString(i)+":COLOR:SECOND_COLOR:A",0.0);

        quads[i].imgColorize.r = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:R",1.0);
        quads[i].imgColorize.g = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:G",1.0);
        quads[i].imgColorize.b = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:B",1.0);
        quads[i].imgColorize.a = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IMG:COLORIZE:A",1.0);

        quads[i].videoColorize.r = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:R",1.0);
        quads[i].videoColorize.g = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:G",1.0);
        quads[i].videoColorize.b = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:B",1.0);
        quads[i].videoColorize.a = XML.getValue("QUADS:QUAD_"+ofToString(i)+":VIDEO:COLORIZE:A",1.0);

        quads[i].camColorize.r = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:R",1.0);
        quads[i].camColorize.g = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:G",1.0);
        quads[i].camColorize.b = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:B",1.0);
        quads[i].camColorize.a = XML.getValue("QUADS:QUAD_"+ofToString(i)+":CAM:COLORIZE:A",1.0);

        quads[i].isOn = XML.getValue("QUADS:QUAD_"+ofToString(i)+":IS_ON",0);

    }
}
