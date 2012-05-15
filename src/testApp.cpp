#include "testApp.h"
#include "stdio.h"
#include <iostream>

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

    ofSetLogLevel(OF_LOG_WARNING);

    // read xml config file
    configOk = XML.loadFile("config.xml");
    if(!configOk) {cout << "WARNING: config file config.xml not found!" << endl << endl;}
    else {cout << "using config file config.xml" << endl;}

    #ifdef WITH_KINECT
    bKinectOk = kinect.setup();
    bCloseKinect = false;
    bOpenKinect = false;
    #endif

    // camera stuff
    cameras.clear();
    numOfCams = 0;
    bCameraOk = False;
    if(configOk)
    {
        XML.pushTag("CAMERAS");
        // check how many cameras are defined in settings
        numOfCams = XML.getNumTags("CAMERA");
        // cycle through defined cameras trying to initialize them and populate the cameras vector
        for (int i=0; i<numOfCams; i++)
        {
            XML.pushTag("CAMERA", i);
            reqCamWidth = XML.getValue("WIDTH",640);
            reqCamHeight = XML.getValue("HEIGHT",480);
            camID = XML.getValue("ID",0);
            XML.popTag();
            ofVideoGrabber cam;
            cam.setDeviceID(camID);
            bCameraOk = cam.initGrabber(reqCamWidth,reqCamHeight);
            camWidth = cam.width;
            camHeight= cam.height;
            string message = "camera with id "+ ofToString(camID) +" asked for %i by %i - actual size is %i by %i \n";
            char *buf = new char[message.length()];
            strcpy(buf,message.c_str());
            printf(buf, reqCamWidth, reqCamHeight, camWidth, camHeight);
            delete []buf;
            // check if the camera is available and eventually push it to cameras vector
            if (camWidth == 0 || camHeight == 0)
            {
                ofSystemAlertDialog("camera with id " + ofToString(camID) + " not found or not available");
            }
            else
            {
                cameras.push_back(cam);
                // following vector is used for the combo box in SimpleGuiToo gui
                cameraIDs.push_back(ofToString(camID));
            }
        }
        XML.popTag();
    }

    // shared videos setup
    sharedVideos.clear();
    for(int i=0; i<3; i++)
    {
        ofVideoPlayer video;
        sharedVideos.push_back(video);
    }

    //double click time
    doubleclickTime = 500;
    lastTap = 0;
    totRotationAngle = 0;

    if(ofGetScreenWidth()>1024 && ofGetScreenHeight()>800 )
    {
        WINDOW_W = 1024;
        WINDOW_H = 768;
    }
    else
    {
        WINDOW_W = 800;
        WINDOW_H = 600;
    }

    //we run at 60 fps!
    //ofSetVerticalSync(true);

    // splash image
    bSplash = true;
    splashImg.loadImage("lpmt_splash.png");
    splashTime = ofGetElapsedTimef();

    //mask editing
    maskSetup = false;

    //grid editing
    gridSetup = false;

    // OSC setup
    if (configOk)
    {
        int oscReceivePort = XML.getValue("OSC:LISTENING_PORT",12345);
        cout << "listening for OSC messages on port: " << oscReceivePort << endl;
        receiver.setup(oscReceivePort);
    }
    else
    {
        cout << "listening for OSC messages on default port 12345" << endl;
        receiver.setup( PORT );
    }
    current_msg_string = 0;

    // we scan the video dir for videos
    //string videoDir = string("./data/video");
    //string videoDir =  ofToDataPath("video",true);
    //videoFiles = vector<string>();
    //getdir(videoDir,videoFiles);
    //string videos[videoFiles.size()];
    //for (unsigned int i = 0; i < videoFiles.size(); i++)
    //{
    //    videos[i]= videoFiles[i];
    //}

    // we scan the slideshow dir for videos
    //string slideshowDir = string("./data/slideshow");
    /*
    string slideshowDir = ofToDataPath("slideshow",true);
    slideshowFolders = vector<string>();
    getdir(slideshowDir,slideshowFolders);
    string slideshows[slideshowFolders.size()];
    for (unsigned int i = 0; i < slideshowFolders.size(); i++)
    {
        slideshows[i]= slideshowFolders[i];
    }
    */

    #ifdef WITH_SYPHON
	// Syphon setup
	syphClient.setup();
    //syphClient.setApplicationName("Simple Server");
    syphClient.setServerName("");
    #endif

    // load shaders
    edgeBlendShader.load("shaders/blend.vert", "shaders/blend.frag");
    quadMaskShader.load("shaders/mask.vert", "shaders/mask.frag");
    chromaShader.load("shaders/chroma.vert", "shaders/chroma.frag");

    //ttf.loadFont("type/frabk.ttf", 11);
    ttf.loadFont("type/OpenSans-Regular.ttf", 11);
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

    // snap mode for surfaces corner is on
    bSnapOn = true;
    // number of surface to use as source in copy/paste is disabled
    copyQuadNum = -1;

    //timeline defaults
    #ifdef WITH_TIMELINE
    useTimeline = false;
    timelineDurationSeconds = timelinePreviousDuration = 10.0;
    #endif

    // texture for snapshot background
    snapshotTexture.allocate(camWidth,camHeight, GL_RGB);
    snapshotOn = 0;


    // initializes layers array
    for(int i = 0; i < 36; i++)
    {
        layers[i] = -1;
    }

    // defines the first 4 default quads
    #ifdef WITH_KINECT
        #ifdef WITH_SYPHON
        quads[0].setup(0.0,0.0,0.5,0.0,0.5,0.5,0.0,0.5, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, kinect, syphClient);
        #else
        quads[0].setup(0.0,0.0,0.5,0.0,0.5,0.5,0.0,0.5, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, kinect);
        #endif
    #else
        #ifdef WITH_SYPHON
        quads[0].setup(0.0,0.0,0.5,0.0,0.5,0.5,0.0,0.5, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, syphClient);
        #else
        quads[0].setup(0.0,0.0,0.5,0.0,0.5,0.5,0.0,0.5, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos);
        #endif
    #endif
    quads[0].quadNumber = 0;
    #ifdef WITH_KINECT
        #ifdef WITH_SYPHON
        quads[1].setup(0.5,0.0,1.0,0.0,1.0,0.5,0.5,0.5, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, kinect, syphClient);
        #else
        quads[1].setup(0.5,0.0,1.0,0.0,1.0,0.5,0.5,0.5, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, kinect);
        #endif
    #else
        #ifdef WITH_SYPHON
        quads[1].setup(0.5,0.0,1.0,0.0,1.0,0.5,0.5,0.5, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, syphClient);
        #else
        quads[1].setup(0.5,0.0,1.0,0.0,1.0,0.5,0.5,0.5, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos);
        #endif
    #endif
    quads[1].quadNumber = 1;
    #ifdef WITH_KINECT
        #ifdef WITH_SYPHON
        quads[2].setup(0.0,0.5,0.5,0.5,0.5,1.0,0.0,1.0, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, kinect, syphClient);
        #else
        quads[2].setup(0.0,0.5,0.5,0.5,0.5,1.0,0.0,1.0, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, kinect);
        #endif
    #else
        #ifdef WITH_SYPHON
        quads[2].setup(0.0,0.5,0.5,0.5,0.5,1.0,0.0,1.0, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, syphClient);
        #else
        quads[2].setup(0.0,0.5,0.5,0.5,0.5,1.0,0.0,1.0, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos);
        #endif
    #endif
    quads[2].quadNumber = 2;
    #ifdef WITH_KINECT
        #ifdef WITH_SYPHON
        quads[3].setup(0.5,0.5,1.0,0.5,1.0,1.0,0.5,1.0, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, kinect, syphClient);
        #else
        quads[3].setup(0.5,0.5,1.0,0.5,1.0,1.0,0.5,1.0, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, kinect);
        #endif
    #else
        #ifdef WITH_SYPHON
        quads[3].setup(0.5,0.5,1.0,0.5,1.0,1.0,0.5,1.0, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, syphClient);
        #else
        quads[3].setup(0.5,0.5,1.0,0.5,1.0,1.0,0.5,1.0, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos);
        #endif
    #endif
    quads[3].quadNumber = 3;
    // define last one as active quad
    activeQuad = 3;
    quads[activeQuad].isActive = True;
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

    // timeline stuff initialization
    #ifdef WITH_TIMELINE
    timelineSetup(timelineDurationSeconds);
    #endif

    // GUI STUFF ---------------------------------------------------

    // general page
    gui.addTitle("show/hide quads");
    // overriding default theme
    //gui.bDrawHeader = false;
    gui.config->toggleHeight = 16;
    gui.config->buttonHeight = 18;
    gui.config->sliderTextHeight = 18;
    gui.config->titleHeight = 18;
    //gui.config->fullActiveColor = 0x6B404B;
    //gui.config->fullActiveColor = 0x5E4D3E;
    gui.config->fullActiveColor = 0x648B96;
    gui.config->textColor = 0xFFFFFF;
    gui.config->textBGOverColor = 0xDB6800;
    // adding controls
    // first a general page for general controls and toggling surfaces on/off
    for(int i = 0; i < 36; i++)
    {
        gui.addToggle("surface "+ofToString(i), quads[i].isOn);
    }
    gui.addTitle("General controls").setNewColumn(true);
    gui.addToggle("surfaces corner snap", bSnapOn);
    gui.addTitle("Shared videos");
    gui.addButton("load shared video 1", bSharedVideoLoad0);
    gui.addButton("load shared video 2", bSharedVideoLoad1);
    gui.addButton("load shared video 3", bSharedVideoLoad2);

    // then three pages of settings for each quad surface
    string blendModesArray[] = {"screen","add","subtract","multiply"};
    for(int i = 0; i < 36; i++)
    {
        gui.addPage("surface "+ofToString(i)+" - 1/3");
        gui.addTitle("surface "+ofToString(i));
        gui.addToggle("show/hide", quads[i].isOn);
        #ifdef WITH_TIMELINE
        gui.addToggle("use timeline", useTimeline);
        gui.addSlider("timeline seconds", timelineDurationSeconds, 10.0, 1200.0);
        gui.addToggle("use timeline col", quads[i].bTimelineColor);
        gui.addToggle("use timeline alpha", quads[i].bTimelineAlpha);
        gui.addToggle("use timeline for slides", quads[i].bTimelineSlideChange);
        #endif
        #ifdef WITH_SYPHON
        gui.addToggle("use Syphon", quads[i].bSyphon);
		gui.addSlider("syphon origin X", quads[i].syphonPosX, -1280, 1280);
        gui.addSlider("syphon origin Y", quads[i].syphonPosY, -1280, 1280);
		gui.addSlider("syphon scale X", quads[i].syphonScaleX, 0.1, 5.0);
        gui.addSlider("syphon scale Y", quads[i].syphonScaleY, 0.1, 5.0);
        #endif
        gui.addToggle("image on/off", quads[i].imgBg);
        gui.addButton("load image", bImageLoad);
        gui.addSlider("img scale X", quads[i].imgMultX, 0.1, 5.0);
        gui.addSlider("img scale Y", quads[i].imgMultY, 0.1, 5.0);
        gui.addToggle("H mirror", quads[i].imgHFlip);
        gui.addToggle("V mirror", quads[i].imgVFlip);
        gui.addColorPicker("img color", &quads[i].imgColorize.r);
        gui.addTitle("Blending modes");
        gui.addToggle("blending on/off", quads[i].bBlendModes);

        gui.addComboBox("blending mode", quads[i].blendMode, 4, blendModesArray);
        gui.addTitle("Solid color").setNewColumn(true);
        gui.addToggle("solid bg on/off", quads[i].colorBg);
        gui.addColorPicker("Color", &quads[i].bgColor.r);
        gui.addToggle("transition color", quads[i].transBg);
        gui.addColorPicker("second Color", &quads[i].secondColor.r);
        gui.addSlider("trans duration", quads[i].transDuration, 0.1, 60.0);
        gui.addTitle("Mask");
        gui.addToggle("mask on/off", quads[i].bMask);
        gui.addToggle("invert mask", quads[i].maskInvert);
        gui.addTitle("Surface deformation");
        gui.addToggle("surface deform.", quads[i].bDeform);
        gui.addToggle("use bezier", quads[i].bBezier);
        gui.addToggle("use grid", quads[i].bGrid);
        gui.addSlider("grid rows", quads[i].gridRows, 2, 15);
        gui.addSlider("grid columns", quads[i].gridColumns, 2, 20);
        gui.addButton("spherize light", bQuadBezierSpherize);
        gui.addButton("spherize strong", bQuadBezierSpherizeStrong);
        gui.addButton("reset bezier warp", bQuadBezierReset);
        gui.addTitle("Edge blending").setNewColumn(true);
        gui.addToggle("edge blend on/off", quads[i].edgeBlendBool);
        gui.addSlider("power", quads[i].edgeBlendExponent, 0.0, 4.0);
        gui.addSlider("gamma", quads[i].edgeBlendGamma, 0.0, 4.0);
        gui.addSlider("luminance", quads[i].edgeBlendLuminance, -4.0, 4.0);
        gui.addSlider("left edge", quads[i].edgeBlendAmountSin, 0.0, 0.5);
        gui.addSlider("right edge", quads[i].edgeBlendAmountDx, 0.0, 0.5);
        gui.addSlider("top edge", quads[i].edgeBlendAmountTop, 0.0, 0.5);
        gui.addSlider("bottom edge", quads[i].edgeBlendAmountBottom, 0.0, 0.5);
        gui.addTitle("Content placement");
        gui.addSlider("X displacement", quads[i].quadDispX, -1280, 1280);
        gui.addSlider("Y displacement", quads[i].quadDispY, -1280, 1280);
        gui.addSlider("Width", quads[i].quadW, 0, 2400);
        gui.addSlider("Height", quads[i].quadH, 0, 2400);
        gui.addButton("Reset", bQuadReset);

        gui.addPage("surface "+ofToString(i)+" - 2/3");
        gui.addTitle("Video");
        gui.addToggle("video on/off", quads[i].videoBg);
        //gui.addComboBox("video bg", quads[i].bgVideo, videoFiles.size(), videos);
        gui.addButton("load video", bVideoLoad);
        gui.addSlider("video scale X", quads[i].videoMultX, 0.1, 5.0);
        gui.addSlider("video scale Y", quads[i].videoMultY, 0.1, 5.0);
        gui.addToggle("H mirror", quads[i].videoHFlip);
        gui.addToggle("V mirror", quads[i].videoVFlip);
        gui.addColorPicker("video color", &quads[i].videoColorize.r);
        gui.addSlider("video sound vol", quads[i].videoVolume, 0, 100);
        gui.addSlider("video speed", quads[i].videoSpeed, -2.0, 4.0);
        gui.addToggle("video loop", quads[i].videoLoop);
        gui.addToggle("video greenscreen", quads[i].videoGreenscreen);
        gui.addToggle("shared video on/off", quads[i].sharedVideoBg);
        gui.addSlider("shared video", quads[i].sharedVideoNum, 1, 3);
        if (cameras.size()>0)
        {
        gui.addTitle("Camera").setNewColumn(true);
        gui.addToggle("cam on/off", quads[i].camBg);
        if(cameras.size()>1)
        {
           gui.addComboBox("select camera", quads[i].camNumber, cameras.size(), &cameraIDs[0]);
        }
        gui.addSlider("camera scale X", quads[i].camMultX, 0.1, 5.0);
        gui.addSlider("camera scale Y", quads[i].camMultY, 0.1, 5.0);
        gui.addToggle("H mirror", quads[i].camHFlip);
        gui.addToggle("V mirror", quads[i].camVFlip);
        gui.addColorPicker("cam color", &quads[i].camColorize.r);
        gui.addToggle("camera greenscreen", quads[i].camGreenscreen);
        gui.addTitle("Greenscreen");
        }
        else
        {
        gui.addTitle("Greenscreen").setNewColumn(true);
        }
        gui.addSlider("g-screen threshold", quads[i].thresholdGreenscreen, 0.0, 255.0);
        gui.addColorPicker("greenscreen col", &quads[i].colorGreenscreen.r);
        gui.addTitle("Slideshow").setNewColumn(false);
        gui.addToggle("slideshow on/off", quads[i].slideshowBg);
        gui.addButton("load slideshow", bSlideshowLoad);
        gui.addSlider("slide duration", quads[i].slideshowSpeed, 0.1, 15.0);
        gui.addToggle("slides to quad size", quads[i].slideFit);
        gui.addToggle("keep aspect ratio", quads[i].slideKeepAspect);
        #ifdef WITH_KINECT
        if(bKinectOk)
        {
            gui.addTitle("Kinect").setNewColumn(true);
            gui.addToggle("use kinect", quads[i].kinectBg);
            gui.addToggle("show kinect image", quads[i].kinectImg);
            gui.addToggle("show kinect gray image", quads[i].getKinectGrayImage);
            gui.addToggle("use kinect as mask", quads[i].kinectMask);
            gui.addToggle("kinect blob detection", quads[i].getKinectContours);
            gui.addToggle("blob curved contour", quads[i].kinectContourCurved);
            gui.addSlider("kinect scale X", quads[i].kinectMultX, 0.1, 5.0);
            gui.addSlider("kinect scale Y", quads[i].kinectMultY, 0.1, 5.0);
            gui.addColorPicker("kinect color", &quads[i].kinectColorize.r);
            gui.addSlider("near threshold", quads[i].nearDepthTh, 0, 255);
            gui.addSlider("far threshold", quads[i].farDepthTh, 0, 255);
            gui.addSlider("kinect tilt angle", kinect.kinectAngle, -30, 30);
            gui.addSlider("kinect image blur", quads[i].kinectBlur, 0, 10);
            gui.addSlider("blob min area", quads[i].kinectContourMin, 0.01, 1.0);
            gui.addSlider("blob max area", quads[i].kinectContourMax, 0.0, 1.0);
            gui.addSlider("blob contour smooth", quads[i].kinectContourSmooth, 0, 20);
            gui.addSlider("blob simplify", quads[i].kinectContourSimplify, 0.0, 2.0);
            gui.addButton("close connection", bCloseKinect);
            gui.addButton("reopen connection", bOpenKinect);

        }
        #endif
        gui.addPage("surface "+ofToString(i)+" - 3/3");
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
        gui.addTitle("Rectangular crop").setNewColumn(true);
        gui.addSlider("top", quads[i].crop[0], 0, 1.0);
        gui.addSlider("right", quads[i].crop[1], 0, 1.0);
        gui.addSlider("bottom", quads[i].crop[2], 0, 1.0);
        gui.addSlider("left", quads[i].crop[3], 0, 1.0);
        gui.addTitle("Circular crop");
        gui.addSlider("center X", quads[i].circularCrop[0], 0, 1.0);
        gui.addSlider("center Y", quads[i].circularCrop[1], 0, 1.0);
        gui.addSlider("radius", quads[i].circularCrop[2], 0, 1.5);
    }

    // then we set displayed gui page to the one corresponding to active quad and show the gui
    gui.setPage((activeQuad*3)+2);
    gui.show();
    // timeline off at start
    bTimeline = false;
    #ifdef WITH_TIMELINE
    timeline.setCurrentPage(ofToString(activeQuad));
    timeline.hide();
    timeline.disable();
    #endif

    // free xml reader from config file
    cout << "setup done! playing now" << endl << endl;
    XML.clear();
}

void testApp::exit()
{

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

        //check if quad dimensions reset button on GUI is pressed
        if(bQuadReset)
        {
            bQuadReset = false;
            quadDimensionsReset(activeQuad);
            quadPlacementReset(activeQuad);
        }

        //check if quad bezier spherize button on GUI is pressed
        if(bQuadBezierSpherize)
        {
            bQuadBezierSpherize = false;
            quadBezierSpherize(activeQuad);
        }

        //check if quad bezier spherize strong button on GUI is pressed
        if(bQuadBezierSpherizeStrong)
        {
            bQuadBezierSpherizeStrong = false;
            quadBezierSpherizeStrong(activeQuad);
        }

        //check if quad bezier reset button on GUI is pressed
        if(bQuadBezierReset)
        {
            bQuadBezierReset = false;
            quadBezierReset(activeQuad);
        }


        // check if image load button on GUI is pressed
        if(bImageLoad)
        {
            bImageLoad = false;
            openImageFile();
        }

        // check if video load button on GUI is pressed
        if(bVideoLoad)
        {
            bVideoLoad = false;
            openVideoFile();
        }

        // check if video load button on GUI is pressed
        if(bSharedVideoLoad0)
        {
            bSharedVideoLoad0 = false;
            openSharedVideoFile(0);
        }
        else if(bSharedVideoLoad1)
        {
            bSharedVideoLoad1 = false;
            openSharedVideoFile(1);
        }
        else if(bSharedVideoLoad2)
        {
            bSharedVideoLoad2 = false;
            openSharedVideoFile(2);
        }

        // check if image load button on GUI is pressed
       if(bSlideshowLoad)
       {
            bSlideshowLoad = false;
            string ssname = loadSlideshow();
            cout << ssname;
            quads[activeQuad].slideshowName = ssname;
        }

        // check if kinect close button on GUI is pressed
        #ifdef WITH_KINECT
        if(bCloseKinect)
        {
            bCloseKinect = false;
            kinect.kinect.setCameraTiltAngle(0);
            kinect.kinect.close();
        }

        // check if kinect close button on GUI is pressed
        if(bOpenKinect)
        {
            bOpenKinect = false;
            kinect.kinect.open();
        }
        #endif

        for (int i=0; i < cameras.size(); i++)
        {
            if (cameras[i].getHeight() > 0)  // isLoaded check
            {
                cameras[i].grabFrame();
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

        // kinect update
        #ifdef WITH_KINECT
        kinect.update();
        #endif

        //timeline update
        #ifdef WITH_TIMELINE
        if(timelineDurationSeconds != timelinePreviousDuration)
        {
            timelinePreviousDuration = timelineDurationSeconds;
            timeline.setDurationInSeconds(timelineDurationSeconds);
        }
        if(useTimeline)
        {
            timelineUpdate();
        }
        #endif

        // loops through initialized quads and runs update, setting the border color as well
        for(int j = 0; j < 36; j++)
        {
            int i = layers[j];
            if (i >= 0)
            {
                if (quads[i].initialized)
                {
                    quads[i].update();
                    quads[i].borderColor = borderColor;
                }
            }
        }

    }
}


//--------------------------------------------------------------
void testApp::dostuff()
{
    if (bStarted)
    {

        // if snapshot is on draws it as window background
        if (isSetup && snapshotOn)
            {
                ofEnableAlphaBlending();
                ofSetHexColor(0xFFFFFF);
                snapshotTexture.draw(0,0,ofGetWidth(),ofGetHeight());
                ofDisableAlphaBlending();
            }

        // loops through initialized quads and calls their draw function
        for(int j = 0; j < 36; j++)
        {
            int i = layers[j];
            if (i >= 0)
            {
                if (quads[i].initialized)
                {
                    quads[i].draw();
                }
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
            // if we are rotating surface, draws a feedback rotation sector
            ofEnableAlphaBlending();
            ofFill();
            ofSetColor(0,200,220,120);
            rotationSector.draw();
            ofNoFill();
            ofDisableAlphaBlending();
            ofSetHexColor(0xFFFFFF);
            ttf.drawString("active surface: "+ofToString(activeQuad), 30, ofGetHeight()-25);
            if(maskSetup) {
                ofSetHexColor(0xFF0000);
                ttf.drawString("Mask-editing mode ", 170, ofGetHeight()-25);
            }
            // draws gui
            gui.draw();
        }
    }

    #ifdef WITH_TIMELINE
    if (bTimeline)
    {
        timeline.draw();
    }
    #endif

    if (bSplash)
    {
        ofEnableAlphaBlending();
        splashImg.draw(((ofGetWidth()/2)-230),((ofGetHeight()/2)-110));
        ofDisableAlphaBlending();
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
    if ( key == '+' && !bTimeline)
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
    if ( key == '-' && !bTimeline)
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
    if ( (key == 's' || key == 'S') && !bTimeline)
    {
        setXml();
        XML.saveFile("_lpmt_settings.xml");
        cout<<"saved settings to data/_lpmt_settings.xml"<<endl;

    }

    // loads settings and quads from default xml file
    if ((key == 'l' || key == 'L') && !bTimeline)
    {
        XML.loadFile("_lpmt_settings.xml");
        getXml();
        cout<<"loaded settings from data/_lpmt_settings.xml"<<endl;
        gui.setPage((activeQuad*3)+2);
    }

    // takes a snapshot of attached camera and uses it as window background
    if (key == 'w' && !bTimeline)
    {
        snapshotOn = !snapshotOn;
        if (snapshotOn == 1)
        {
            cameras[0].grabFrame();
            snapshotTexture.allocate(camWidth,camHeight, GL_RGB);
            unsigned char * pixels = cameras[0].getPixels();
            snapshotTexture.loadData(pixels, camWidth,camHeight, GL_RGB);
        }
    }

    // takes a snapshot from an image file and uses it as window background
    if (key == 'W' && !bTimeline)
    {
        snapshotOn = !snapshotOn;
        if (snapshotOn == 1)
        {
            ofImage img;
            img.clone(loadImageFromFile());
            snapshotTexture.allocate(img.width,img.height, GL_RGB);
            unsigned char * pixels = img.getPixels();
            snapshotTexture.loadData(pixels, img.width,img.height, GL_RGB);
        }
    }

    // fills window with active quad
    if ( (key =='q' || key == 'Q') && !bTimeline)
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
    if ( key =='>' && !bTimeline)
    {
        if (isSetup)
        {
            quads[activeQuad].isActive = False;
            activeQuad += 1;
            if (activeQuad > nOfQuads-1)
            {
                activeQuad = 0;
            }
            quads[activeQuad].isActive = True;
        }
        gui.setPage((activeQuad*3)+2);
    }

    // activates prev quad
    if ( key =='<' && !bTimeline)
    {
        if (isSetup)
        {
            quads[activeQuad].isActive = False;
            activeQuad -= 1;
            if (activeQuad < 0)
            {
                activeQuad = nOfQuads-1;
            }
            quads[activeQuad].isActive = True;
        }
        gui.setPage((activeQuad*3)+2);
    }

    // goes to first page of gui for active quad or, in mask edit mode, delete last drawn point
    if ( (key == 'z' || key == 'Z') && !bTimeline)
    {
        if(maskSetup && quads[activeQuad].maskPoints.size()>0) {quads[activeQuad].maskPoints.pop_back();}
        else {gui.setPage((activeQuad*3)+2);}
    }

    if ( key == OF_KEY_F1)
    {
        gui.setPage((activeQuad*3)+2);
    }

    if ( (key == 'd' || key == 'D') && !bTimeline)
    {
        if(maskSetup && quads[activeQuad].maskPoints.size()>0)
        {
            if (quads[activeQuad].bHighlightMaskPoint)
            {
                quads[activeQuad].maskPoints.erase(quads[activeQuad].maskPoints.begin()+quads[activeQuad].highlightedMaskPoint);
            }

        }
    }


    // goes to second page of gui for active quad
    if ( (key == 'x' || key == 'X' || key == OF_KEY_F2) && !bTimeline)
    {
        gui.setPage((activeQuad*3)+3);
    }

    // goes to second page of gui for active quad or, in edit mask mode, clears mask
    if ( (key == 'c' || key == 'C') && !bTimeline)
    {
        if(maskSetup) {quads[activeQuad].maskPoints.clear();}
        else {gui.setPage((activeQuad*3)+4);}
    }

    if (key == OF_KEY_F3)
    {
        gui.setPage((activeQuad*3)+4);
    }

    // paste settings from source surface to current active surface
    /*if ( (key == 'v') && !bTimeline)
    {
        if(glutGetModifiers() & GLUT_ACTIVE_CTRL)
        {
           copyQuadSettings(copyQuadNum);
        }
    }*/

    if ( (key == 3) && !bTimeline)
    {
        copyQuadNum = activeQuad;
    }


    if ( (key == 22) && !bTimeline)
    {
        copyQuadSettings(copyQuadNum);
    }

    // adds a new quad in the middle of the screen
    if ( key =='a' && !bTimeline)
    {
        if (isSetup)
        {
            if (nOfQuads < 36)
            {
                #ifdef WITH_KINECT
                    #ifdef WITH_SYPHON
                    quads[nOfQuads].setup(0.25,0.25,0.75,0.25,0.75,0.75,0.25,0.75, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, kinect, syphClient);
                    #else
                    quads[nOfQuads].setup(0.25,0.25,0.75,0.25,0.75,0.75,0.25,0.75, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, kinect);
                    #endif
                #else
                    #ifdef WITH_SYPHON
                    quads[nOfQuads].setup(0.25,0.25,0.75,0.25,0.75,0.75,0.25,0.75, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos, syphClient);
                    #else
                    quads[nOfQuads].setup(0.25,0.25,0.75,0.25,0.75,0.75,0.25,0.75, edgeBlendShader, quadMaskShader, chromaShader, cameras, sharedVideos);
                    #endif
                #endif
                quads[nOfQuads].quadNumber = nOfQuads;
                layers[nOfQuads] = nOfQuads;
                quads[nOfQuads].layer = nOfQuads;
                quads[activeQuad].isActive = False;
                quads[nOfQuads].isActive = True;
                activeQuad = nOfQuads;
                ++nOfQuads;
                gui.setPage((activeQuad*3)+2);
                // add timeline page for new quad
                #ifdef WITH_TIMELINE
                timelineAddQuadPage(activeQuad);
                #endif
                // next line fixes a bug i've been tracking down for a looong time
                glDisable(GL_DEPTH_TEST);

            }
        }
    }

    // toggles setup mode
    if ( key ==' ' && !bTimeline)
    {
        if (isSetup)
        {
            isSetup = False;
            gui.hide();
            bGui = False;
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
            gui.show();
            bGui = True;
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
    if(key == 'f' && !bTimeline)
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
    if(key == 'g' && !bTimeline)
    {
        if (maskSetup) {
            maskSetup = False;
            for(int i = 0; i < 36; i++)
                {
                    if (quads[i].initialized)
                    {
                        quads[i].isMaskSetup = False;
                    }
                }
        }
        gui.toggleDraw();
        bGui = !bGui;
    }

    // toggles mask editing
    if(key == 'm' && !bTimeline)
    {
        if (!bGui){
        maskSetup = !maskSetup;
        for(int i = 0; i < 36; i++)
            {
                if (quads[i].initialized)
                {
                    quads[i].isMaskSetup = !quads[i].isMaskSetup;
                }
            }
        }
    }

    // toggles bezier deformation editing
    if(key == 'b' && !bTimeline)
    {
        if (!bGui){
        gridSetup = !gridSetup;
        for(int i = 0; i < 36; i++)
            {
                if (quads[i].initialized)
                {
                    quads[i].isBezierSetup = !quads[i].isBezierSetup;
                }
            }
        }
    }

    if(key == '[' && !bTimeline)
    {
        gui.prevPage();
    }

    if(key == ']' && !bTimeline)
    {
        gui.nextPage();
    }

    // show general settings page of gui
    if(key == '1' && !bTimeline)
    {
        gui.setPage(1);
    }

    // resyncs videos to start point in every quad
    if((key == 'r' || key == 'R') && !bTimeline)
    {
        resync();
    }


    // starts and stops rendering

    if(key == 'p' && !bTimeline)
    {
        startProjection();
    }

    if(key == 'o' && !bTimeline)
    {
        stopProjection();
    }

    if(key == 'n' && !bTimeline)
    {
        mpeSetup();
    }

    // displays help in system dialog
    if(key == 'h' && !bTimeline)
    {
        ofBuffer buf = ofBufferFromFile("help_keys.txt");
        ofSystemAlertDialog(buf);
    }

    // show-hide stage when timeline is shown
    if(key == OF_KEY_F11 && bTimeline)
    {
        if(bStarted)
        {
            bStarted = false;
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
        else if(!bStarted)
        {
            bStarted = true;
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
    }


    // toggle timeline
    #ifdef WITH_TIMELINE
    if(key == OF_KEY_F10)
    {
        bTimeline = !bTimeline;
        timeline.toggleShow();
        if(bTimeline)
        {
            timeline.enable();
            gui.hide();
            bGui = false;
        }
        else
        {
            timeline.disable();
        }
    }

    // toggle timeline playing
    if(key == OF_KEY_F12)
    {
        timeline.togglePlay();
    }

    // toggle timeline BPM grid drawing
    if(key == OF_KEY_F9 && bTimeline)
    {
        timeline.toggleDrawBPMGrid();
    }
    #endif

    if(key == '*' && !bTimeline)
    {
        if(cameras[quads[activeQuad].camNumber].getPixelFormat() == OF_PIXELS_RGBA)
        {
            cameras[quads[activeQuad].camNumber].setPixelFormat(OF_PIXELS_BGRA);
        }
        else if(cameras[quads[activeQuad].camNumber].getPixelFormat() == OF_PIXELS_BGRA)
        {
            cameras[quads[activeQuad].camNumber].setPixelFormat(OF_PIXELS_RGBA);
        }

    }

    // rotation of surface around its center
    if(key == '3' && !bTimeline)
    {
        ofMatrix4x4 rotation;
        ofMatrix4x4 centerToOrigin;
        ofMatrix4x4 originToCenter;
        ofMatrix4x4 resultingMatrix;
        centerToOrigin.makeTranslationMatrix(-quads[activeQuad].center);
        originToCenter.makeTranslationMatrix(quads[activeQuad].center);
        rotation.makeRotationMatrix(-5.0,0,0,1);
        resultingMatrix = centerToOrigin * rotation * originToCenter;
        for(int i=0; i<4; i++)
        {
            quads[activeQuad].corners[i] = quads[activeQuad].corners[i] * resultingMatrix;
        }
    }

    if(key == '4' && !bTimeline)
    {
        ofMatrix4x4 rotation;
        ofMatrix4x4 centerToOrigin;
        ofMatrix4x4 originToCenter;
        ofMatrix4x4 resultingMatrix;
        centerToOrigin.makeTranslationMatrix(-quads[activeQuad].center);
        originToCenter.makeTranslationMatrix(quads[activeQuad].center);
        rotation.makeRotationMatrix(5.0,0,0,1);
        resultingMatrix = centerToOrigin * rotation * originToCenter;
        for(int i=0; i<4; i++)
        {
            quads[activeQuad].corners[i] = quads[activeQuad].corners[i] * resultingMatrix;
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

    if (isSetup && !bGui && !maskSetup && !gridSetup && !bTimeline)
    {
        float smallestDist = 1.0;
        whichCorner = -1;

        for(int i = 0; i < 4; i++)
        {
            float distx = quads[activeQuad].corners[i].x - (float)x/ofGetWidth();
            float disty = quads[activeQuad].corners[i].y - (float)y/ofGetHeight();
            float dist  = sqrt( distx * distx + disty * disty);

            if(dist < smallestDist && dist < 0.05) // value for dist threshold can vary between 0.1-0.05, fine tune it
            {
                whichCorner = i;
                smallestDist = dist;
            }
        }

        if(whichCorner >= 0)
            {
                quads[activeQuad].bHighlightCorner = True;
                quads[activeQuad].highlightedCorner = whichCorner;
            }
        else
            {
                quads[activeQuad].bHighlightCorner = False;
                quads[activeQuad].highlightedCorner = -1;

                // distance from center
                float distx = quads[activeQuad].center.x - (float)x / ofGetWidth();
                float disty = quads[activeQuad].center.y - (float)y/ofGetHeight();
                float dist  = sqrt( distx * distx + disty * disty);
                if(dist < 0.05)
                {
                    quads[activeQuad].bHighlightCenter = true;
                }
                else {quads[activeQuad].bHighlightCenter = false;}

                // distance from rotation grab point
                ofPoint rotationGrabPoint;
                //rotationGrabPoint.x = (quads[activeQuad].corners[2].x - quads[activeQuad].corners[1].x)/2 + quads[activeQuad].corners[1].x;
                //rotationGrabPoint.y = (quads[activeQuad].corners[2].y - quads[activeQuad].corners[1].y)/2 + quads[activeQuad].corners[1].y;
                //rotationGrabPoint = ((quads[activeQuad].corners[2]+quads[activeQuad].corners[1])/2+quads[activeQuad].center)/2;
                rotationGrabPoint = (quads[activeQuad].center);
                rotationGrabPoint.x = rotationGrabPoint.x + 0.1;
                float rotationDistx = rotationGrabPoint.x - (float)x / ofGetWidth();
                float rotationDisty = rotationGrabPoint.y - (float)y/ofGetHeight();
                float rotationDist = sqrt(rotationDistx*rotationDistx + rotationDisty*rotationDisty);
                if(rotationDist < 0.05)
                {
                    quads[activeQuad].bHighlightRotation = true;
                }
                else {quads[activeQuad].bHighlightRotation = false;}
            }
    }

    else if (maskSetup && !gridSetup && !bTimeline)
    {
        float smallestDist = sqrt( ofGetWidth() * ofGetWidth() + ofGetHeight() * ofGetHeight());;
        int whichPoint = -1;
        ofVec3f warped;
        for(int i = 0; i < quads[activeQuad].maskPoints.size(); i++)
        {
            warped = quads[activeQuad].getWarpedPoint(x,y);
            float distx = (float)quads[activeQuad].maskPoints[i].x - (float)warped.x;
            float disty = (float)quads[activeQuad].maskPoints[i].y - (float)warped.y;
            float dist  = sqrt( distx * distx + disty * disty);

            if(dist < smallestDist && dist < 20.0)
            {
                whichPoint = i;
                smallestDist = dist;
            }
        }
        if(whichPoint >= 0)
            {
                quads[activeQuad].bHighlightMaskPoint = True;
                quads[activeQuad].highlightedMaskPoint = whichPoint;
            }
        else
            {
                quads[activeQuad].bHighlightMaskPoint = False;
                quads[activeQuad].highlightedMaskPoint = -1;
            }
    }

    else if (gridSetup && !maskSetup && !bTimeline)
    {
        float smallestDist = sqrt( ofGetWidth() * ofGetWidth() + ofGetHeight() * ofGetHeight());;
        int whichPointRow = -1;
        int whichPointCol = -1;
        ofVec3f warped;

        if(quads[activeQuad].bBezier)
        {
        for(int i = 0; i < 4; i++)
        {
            for (int j = 0; j < 4; j++)
            {
                warped = quads[activeQuad].getWarpedPoint(x,y);
                float distx = (float)quads[activeQuad].bezierPoints[i][j][0] * ofGetWidth() - (float)warped.x;
                float disty = (float)quads[activeQuad].bezierPoints[i][j][1] * ofGetHeight() - (float)warped.y;
                float dist  = sqrt( distx * distx + disty * disty);

                if(dist < smallestDist && dist < 20.0)
                {
                    whichPointRow = i;
                    whichPointCol = j;
                    smallestDist = dist;
                }
            }
        }
        }

        else if(quads[activeQuad].bGrid)
        {
        for(int i = 0; i <= quads[activeQuad].gridRows; i++)
        {
            for (int j = 0; j <= quads[activeQuad].gridColumns; j++)
            {
                warped = quads[activeQuad].getWarpedPoint(x,y);
                float distx = (float)quads[activeQuad].gridPoints[i][j][0] * ofGetWidth() - (float)warped.x;
                float disty = (float)quads[activeQuad].gridPoints[i][j][1] * ofGetHeight() - (float)warped.y;
                float dist  = sqrt( distx * distx + disty * disty);

                if(dist < smallestDist && dist < 20.0)
                {
                    whichPointRow = i;
                    whichPointCol = j;
                    smallestDist = dist;
                }
            }
        }
        }

        if(whichPointRow >= 0)
            {
                quads[activeQuad].bHighlightCtrlPoint = True;
                quads[activeQuad].highlightedCtrlPointRow = whichPointRow;
                quads[activeQuad].highlightedCtrlPointCol = whichPointCol;
            }
        else
            {
                quads[activeQuad].bHighlightCtrlPoint = False;
                quads[activeQuad].highlightedCtrlPointRow = -1;
                quads[activeQuad].highlightedCtrlPointCol = -1;
            }
    }
}

//--------------------------------------------------------------
void testApp::mouseDragged(int x, int y, int button)
{
    if (isSetup && !bGui && !maskSetup && !gridSetup && !bTimeline)
    {

        float scaleX = (float)x / ofGetWidth();
        float scaleY = (float)y / ofGetHeight();

        if(whichCorner >= 0)
        {
            quads[activeQuad].corners[whichCorner].x = scaleX;
            quads[activeQuad].corners[whichCorner].y = scaleY;
        }
        // check if we can move or rotate whole quad by dragging its center and rotation mark
        else
        {
            if(quads[activeQuad].bHighlightCenter) // TODO: verifiy if threshold value is good for distance
            {
                ofPoint mouse;
                ofPoint movement;
                mouse.x = x;
                mouse.y = y;
                movement = mouse-startDrag;

                for(int i = 0; i < 4; i++)
                {
                    quads[activeQuad].corners[i].x = quads[activeQuad].corners[i].x + ((float)movement.x / ofGetWidth());
                    quads[activeQuad].corners[i].y = quads[activeQuad].corners[i].y + ((float)movement.y / ofGetHeight());
                }
                startDrag.x = x;
                startDrag.y = y;
            }

            else if(quads[activeQuad].bHighlightRotation)
            {
                float angle;
                ofPoint mouse;
                ofPoint center;
                mouse.x = x;
                mouse.y = y;
                center.x = quads[activeQuad].center.x * ofGetWidth();
                center.y = quads[activeQuad].center.y * ofGetHeight();
                ofVec3f vec1 = (startDrag-center);
                ofVec3f vec2 = (mouse-center);
                angle = ofRadToDeg(atan2(vec2.y,vec2.x) - atan2(vec1.y,vec1.x));

                totRotationAngle += angle;
                rotationSector.clear();
                rotationSector.addVertex(center);
                rotationSector.lineTo(center.x+(0.025*ofGetWidth()),center.y);
                rotationSector.arc(center, 0.025*ofGetWidth(), 0.025*ofGetWidth(), 0, totRotationAngle, 40);
                rotationSector.close();

                ofMatrix4x4 rotation;
                ofMatrix4x4 centerToOrigin;
                ofMatrix4x4 originToCenter;
                ofMatrix4x4 resultingMatrix;
                centerToOrigin.makeTranslationMatrix(-quads[activeQuad].center);
                originToCenter.makeTranslationMatrix(quads[activeQuad].center);
                rotation.makeRotationMatrix(angle,0,0,1);
                resultingMatrix = centerToOrigin * rotation * originToCenter;
                for(int i=0; i<4; i++)
                {
                    quads[activeQuad].corners[i] = quads[activeQuad].corners[i] * resultingMatrix;
                }
                startDrag.x = x;
                startDrag.y = y;
            }
        }
    }
    else if(maskSetup && quads[activeQuad].bHighlightMaskPoint && !bTimeline)
    {
        ofVec3f punto;
        punto = quads[activeQuad].getWarpedPoint(x,y);
        quads[activeQuad].maskPoints[quads[activeQuad].highlightedMaskPoint].x = punto.x;
        quads[activeQuad].maskPoints[quads[activeQuad].highlightedMaskPoint].y = punto.y;
    }

    else if(gridSetup && quads[activeQuad].bHighlightCtrlPoint && !bTimeline)
    {
        if(quads[activeQuad].bBezier)
        {
        ofVec3f punto;
        punto = quads[activeQuad].getWarpedPoint(x,y);
        quads[activeQuad].bezierPoints[quads[activeQuad].highlightedCtrlPointRow][quads[activeQuad].highlightedCtrlPointCol][0] = (float)punto.x/ofGetWidth();
        quads[activeQuad].bezierPoints[quads[activeQuad].highlightedCtrlPointRow][quads[activeQuad].highlightedCtrlPointCol][1] = (float)punto.y/ofGetHeight();
        }
        else if(quads[activeQuad].bGrid)
        {
        ofVec3f punto;
        punto = quads[activeQuad].getWarpedPoint(x,y);
        quads[activeQuad].gridPoints[quads[activeQuad].highlightedCtrlPointRow][quads[activeQuad].highlightedCtrlPointCol][0] = (float)punto.x/ofGetWidth();
        quads[activeQuad].gridPoints[quads[activeQuad].highlightedCtrlPointRow][quads[activeQuad].highlightedCtrlPointCol][1] = (float)punto.y/ofGetHeight();
        }
    }
}

//--------------------------------------------------------------
void testApp::mousePressed(int x, int y, int button)
{
    rotationSector.clear();
    // this is used for dragging the whole quad using its centroid
    startDrag.x = x;
    startDrag.y = y;

    if(bSplash)
    {
        bSplash = !bSplash;
    }

    if (isSetup && !bGui && !bTimeline)
    {

        if(maskSetup && !gridSetup) {
            if (!quads[activeQuad].bHighlightMaskPoint)
            {
                quads[activeQuad].maskAddPoint(x, y);
            }
        }

        else {
        float smallestDist = 1.0;
        whichCorner = -1;
        unsigned long curTap = ofGetElapsedTimeMillis();
        if(lastTap != 0 && curTap - lastTap < doubleclickTime){
            activateQuad(x,y);
        }
        lastTap = curTap;

        // check if we are near once of active quad's corners
        for(int i = 0; i < 4; i++)
        {
            float distx = quads[activeQuad].corners[i].x - (float)x/ofGetWidth();
            float disty = quads[activeQuad].corners[i].y - (float)y/ofGetHeight();
            float dist  = sqrt( distx * distx + disty * disty);

            if(dist < smallestDist && dist < 0.05)
            {
                whichCorner = i;
                smallestDist = dist;
            }
        }
    }
    }
}

//--------------------------------------------------------------
void testApp::mouseReleased()
{
    totRotationAngle = 0;
    rotationSector.clear();
    if (isSetup && !bGui && !bTimeline)
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
        if (snapQuad >= 0 && snapCorner >= 0 && bSnapOn)
        {
            quads[activeQuad].corners[whichCorner].x = quads[snapQuad].corners[snapCorner].x;
            quads[activeQuad].corners[whichCorner].y = quads[snapQuad].corners[snapCorner].y;
        }
    }
    whichCorner = -1;
    quads[activeQuad].bHighlightCorner = False;
    }
}


void testApp::windowResized(int w, int h)
{
            #ifdef WITH_TIMELINE
            timeline.setWidth(w);
            #endif
            for(int i = 0; i < 36; i++)
            {
                if (quads[i].initialized)
                {
                    quads[i].bHighlightCorner = False;
                    quads[i].allocateFbo(ofGetWidth(),ofGetHeight());
                    quadDimensionsReset(i);
                }
            }
}



//---------------------------------------------------------------
void testApp::quadDimensionsReset(int q)
{
    quads[q].quadW = ofGetWidth();
    quads[q].quadH = ofGetHeight();
}

//---------------------------------------------------------------
void testApp::quadPlacementReset(int q)
{
    quads[q].quadDispX = 0;
    quads[q].quadDispY = 0;
}

//---------------------------------------------------------------
void testApp::quadBezierSpherize(int q)
{
    float w = (float)ofGetWidth();
    float h = (float)ofGetHeight();
    float k = (sqrt(2)-1)*4/3;
    quads[q].bBezier = true;

    float tmp_bezierPoints[4][4][3] =
    {
        {   {0*h/w+(0.5*(w/h-1))*h/w, 0, 0},{0.5*k*h/w+(0.5*(w/h-1))*h/w, -0.5*k, 0},    {(1.0*h/w)-(0.5*k*h/w)+(0.5*(w/h-1))*h/w, -0.5*k, 0},    {1.0*h/w+(0.5*(w/h-1))*h/w, 0, 0}    },
        {   {0*h/w-(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 0.5*k, 0},        {0*h/w+(0.5*(w/h-1))*h/w, 0, 0},  {1.0*h/w+(0.5*(w/h-1))*h/w, 0, 0},  {1.0*h/w+(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 0.5*k, 0}  },
        {   {0*h/w-(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 1.0-0.5*k, 0},        {0*h/w+(0.5*(w/h-1))*h/w, 1.0, 0},  {1.0*h/w+(0.5*(w/h-1))*h/w, 1.0, 0},  {1.0*h/w+(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 1.0-0.5*k, 0}  },
        {   {0*h/w+(0.5*(w/h-1))*h/w, 1.0, 0}, {0.5*k*h/w+(0.5*(w/h-1))*h/w, 1.0+0.5*k, 0},  {(1.0*h/w)-(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 1.0+0.5*k, 0},  {1.0*h/w+(0.5*(w/h-1))*h/w, 1.0, 0}  }
    };

	for (int i = 0; i < 4; ++i) {
		for (int j = 0; j < 4; ++j) {
			for (int k = 0; k < 3; ++k) {
				 quads[q].bezierPoints [i][j][k] = tmp_bezierPoints[i][j][k];
			}
		}
	}
	/*  quads[q].bezierPoints =
    {
        {   {(0.5*w/h-0.5)*h/w, 0, 0},  {0.5*(k+w/h-1)*h/w, -0.5*k, 0},    {0.5*(1-k+w/h)*h/w, -0.5*k, 0},    {1.0*h/w+(0.5*(w/h-1))*h/w, 0, 0}    },
        {   {0*h/w-(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 0.5*k, 0},        {0*h/w+(0.5*(w/h-1))*h/w, 0, 0},  {1.0*h/w+(0.5*(w/h-1))*h/w, 0, 0},  {1.0*h/w+(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 0.5*k, 0}  },
        {   {0*h/w-(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 1.0-0.5*k, 0},        {0*h/w+(0.5*(w/h-1))*h/w, 1.0, 0},  {1.0*h/w+(0.5*(w/h-1))*h/w, 1.0, 0},  {1.0*h/w+(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 1.0-0.5*k, 0}  },
        {   {0*h/w+(0.5*(w/h-1))*h/w, 1.0, 0},        {0.5*k*h/w+(0.5*(w/h-1))*h/w, 1.0+0.5*k, 0},  {(1.0*h/w)-(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 1.0+0.5*k, 0},  {1.0*h/w+(0.5*(w/h-1))*h/w, 1.0, 0}  }
    }; */
}

//---------------------------------------------------------------
void testApp::quadBezierSpherizeStrong(int q)
{
    float w = (float)ofGetWidth();
    float h = (float)ofGetHeight();
    float k = (sqrt(2)-1)*4/3;
    quads[q].bBezier = true;

    float tmp_bezierPoints[4][4][3] =
    {
        {   {0*h/w+(0.5*(w/h-1))*h/w, 0, 0},  {0.5*k*h/w+(0.5*(w/h-1))*h/w, -0.5*k, 0},    {(1.0*h/w)-(0.5*k*h/w)+(0.5*(w/h-1))*h/w, -0.5*k, 0},    {1.0*h/w+(0.5*(w/h-1))*h/w, 0, 0}    },
        {   {0*h/w-(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 0.5*k, 0},        {0*h/w-(0.5*k*h/w)+(0.5*(w/h-1))*h/w, -0.5*k, 0},  {1.0*h/w+(0.5*k*h/w)+(0.5*(w/h-1))*h/w, -0.5*k, 0},  {1.0*h/w+(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 0.5*k, 0}  },
        {   {0*h/w-(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 1.0-0.5*k, 0},        {0*h/w-(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 1.0+0.5*k, 0},  {1.0*h/w+(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 1.0+0.5*k, 0},  {1.0*h/w+(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 1.0-0.5*k, 0}  },
        {   {0*h/w+(0.5*(w/h-1))*h/w, 1.0, 0},        {0.5*k*h/w+(0.5*(w/h-1))*h/w, 1.0+0.5*k, 0},  {(1.0*h/w)-(0.5*k*h/w)+(0.5*(w/h-1))*h/w, 1.0+0.5*k, 0},  {1.0*h/w+(0.5*(w/h-1))*h/w, 1.0, 0}  }
    };

	for (int i = 0; i < 4; ++i) {
		for (int j = 0; j < 4; ++j) {
			for (int k = 0; k < 3; ++k) {
				quads[q].bezierPoints [i][j][k] = tmp_bezierPoints[i][j][k];
			}
		}
	}
}

//---------------------------------------------------------------
void testApp::quadBezierReset(int q)
{
    quads[q].bBezier = true;
    float tmp_bezierPoints[4][4][3] =
    {
        {   {0, 0, 0},          {0.333, 0, 0},    {0.667, 0, 0},    {1.0, 0, 0}    },
        {   {0, 0.333, 0},        {0.333, 0.333, 0},  {0.667, 0.333, 0},  {1.0, 0.333, 0}  },
        {   {0, 0.667, 0},        {0.333, 0.667, 0},  {0.667, 0.667, 0},  {1.0, 0.667, 0}  },
        {   {0, 1.0, 0},        {0.333, 1.0, 0},  {0.667, 1.0, 0},  {1.0, 1.0, 0}  }
    };

	for (int i = 0; i < 4; ++i) {
		for (int j = 0; j < 4; ++j) {
			for (int k = 0; k < 3; ++k) {
				quads[q].bezierPoints [i][j][k] = tmp_bezierPoints[i][j][k];
			}
		}
	}
}


//---------------------------------------------------------------
void testApp::activateQuad(int x, int y)
{
    float smallestDist = 1.0;
    int whichQuad = activeQuad;

    for(int i = 0; i < 36; i++)
    {
        if (quads[i].initialized)
        {
            float distx = quads[i].center.x - (float)x/ofGetWidth();
            float disty = quads[i].center.y - (float)y/ofGetHeight();
            float dist  = sqrt( distx * distx + disty * disty);

            if(dist < smallestDist && dist < 0.1)
            {
                whichQuad = i;
                smallestDist = dist;
            }
        }
    }
    if (whichQuad != activeQuad)
    {
        quads[activeQuad].isActive = False;
        activeQuad = whichQuad;
        quads[activeQuad].isActive = True;
        gui.setPage((activeQuad*3)+2);
    }
}
