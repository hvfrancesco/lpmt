#include "testApp.h"

//-----------------------------------------------------------
void testApp::openImageFile()
{
    cout << "loading image " << endl;
    ofFileDialogResult dialog_result = ofSystemLoadDialog("load image file");
    cout << "image loaded " << endl;
    if(dialog_result.bSuccess)
    {
        quads[activeQuad].loadImageFromFile(dialog_result.getName(), dialog_result.getPath());
    }
}

//-----------------------------------------------------------
void testApp::openVideoFile()
{
    cout << "loading video " << endl;
    ofFileDialogResult dialog_result = ofSystemLoadDialog("load video file");
    cout << "video loaded " << endl;
    if(dialog_result.bSuccess)
    {
        quads[activeQuad].loadVideoFromFile(dialog_result.getName(), dialog_result.getPath());
    }
}

//-----------------------------------------------------------
void testApp::openSharedVideoFile(int i)
{
    cout << "loading shared video " << endl;
    ofFileDialogResult dialog_result = ofSystemLoadDialog("load shared video file");
    if(dialog_result.bSuccess)
    {
        if (sharedVideos[i].isLoaded())
        {
            sharedVideos[i].closeMovie();
        }
        string path = dialog_result.getPath();
        sharedVideos[i].loadMovie(path);
        if(sharedVideos[i].isLoaded())
        {
            cout << "shared video loaded" << endl;
            sharedVideosFiles[i] = path;
            sharedVideos[i].setLoopState(OF_LOOP_NORMAL);
            sharedVideos[i].play();
            sharedVideos[i].setVolume(0);
            for(int j=0; j<36; j++)
            {
                if (quads[j].initialized)
                {
                    quads[j].vids[i] = sharedVideos[i];
                }
            }
        }
    }
}

//-----------------------------------------------------------
void testApp::openSharedVideoFile(string path, int i)
{
        if (sharedVideos[i].isLoaded())
        {
            sharedVideos[i].closeMovie();
        }
        sharedVideos[i].loadMovie(path);
        if(sharedVideos[i].isLoaded())
        {
            cout << "shared video loaded" << endl;
            sharedVideosFiles[i] = path;
            sharedVideos[i].setLoopState(OF_LOOP_NORMAL);
            sharedVideos[i].play();
            sharedVideos[i].setVolume(0);
            for(int j=0; j<36; j++)
            {
                if (quads[j].initialized)
                {
                    quads[j].vids[i] = sharedVideos[i];
                }
            }
        }
}


//-----------------------------------------------------------
ofImage testApp::loadImageFromFile()
{
    ofFileDialogResult dialog_result = ofSystemLoadDialog("load image file", false);
    if(dialog_result.bSuccess)
    {
        ofImage img;
        string imgName = dialog_result.getName();
        string imgPath = dialog_result.getPath();
        ofFile image(imgPath);
        img.loadImage(image);
        return img;
    }

}

//-----------------------------------------------------------

string testApp::loadSlideshow()
{
   ofFileDialogResult dialog_result = ofSystemLoadDialog("find slideshow folder", true);
    if(dialog_result.bSuccess)
    {
        string slideshowPath = dialog_result.getPath();
        return slideshowPath;
   }
}

//--------------------------------------------------------------
void testApp::resync()
{
    #ifdef WITH_TIMELINE
    if(useTimeline)
    {
        timeline.setCurrentTimeSeconds(0.0);
    }
    #endif

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
    for(int i=0; i<8; i++)
    {
        if(sharedVideos[i].isLoaded())
        {
            sharedVideos[i].setPosition(0.0);
        }
    }
}

//--------------------------------------------------------------
void testApp::startProjection()
{
    bStarted = True;
    #ifdef WITH_TIMELINE
    if(useTimeline)
    {
        timeline.enable();
        timeline.play();
    }
    #endif
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
    for(int i=0; i<8; i++)
    {
        if(sharedVideos[i].isLoaded())
        {
            sharedVideos[i].play();
        }
    }
}

//--------------------------------------------------------------
void testApp::stopProjection()
{
    bStarted = False;
    #ifdef WITH_TIMELINE
    if(useTimeline)
    {
        timeline.stop();
        timeline.hide();
        timeline.disable();
    }
    #endif
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
    for(int i=0; i<8; i++)
    {
        if(sharedVideos[i].isLoaded())
        {
            sharedVideos[i].stop();
        }
    }
}

//--------------------------------------------------------------
void testApp::copyQuadSettings(int sourceQuad)
{
    if(sourceQuad >= 0)
    {
       quads[activeQuad].quadDispX = quads[sourceQuad].quadDispX;
    quads[activeQuad].quadDispY = quads[sourceQuad].quadDispY;
    quads[activeQuad].quadW = quads[sourceQuad].quadW;
    quads[activeQuad].quadH = quads[sourceQuad].quadH;
    quads[activeQuad].imgBg = quads[sourceQuad].imgBg;
    //quads[activeQuad].loadedImg = quads[sourceQuad].loadedImg;
    quads[activeQuad].bgImg = quads[sourceQuad].bgImg;
    if ((quads[activeQuad].imgBg) && (quads[activeQuad].bgImg != ""))
    {
        quads[activeQuad].loadImageFromFile(quads[activeQuad].loadedImg, quads[activeQuad].bgImg);
    }
    quads[activeQuad].imgHFlip = quads[sourceQuad].imgHFlip;
    quads[activeQuad].imgVFlip = quads[sourceQuad].imgVFlip;
    quads[activeQuad].videoBg = quads[sourceQuad].videoBg;
    //quads[activeQuad].loadedVideo = quads[sourceQuad].loadedVideo;
    quads[activeQuad].bgVideo = quads[sourceQuad].bgVideo;
    if ((quads[activeQuad].videoBg) && (quads[activeQuad].bgVideo != ""))
    {
        quads[activeQuad].loadVideoFromFile(quads[activeQuad].loadedVideo, quads[activeQuad].bgVideo);
    }
    quads[activeQuad].videoHFlip = quads[sourceQuad].videoHFlip;
    quads[activeQuad].videoVFlip = quads[sourceQuad].videoVFlip;
    quads[activeQuad].bgSlideshow = quads[sourceQuad].bgSlideshow;
    quads[activeQuad].colorBg = quads[sourceQuad].colorBg;
    quads[activeQuad].transBg = quads[sourceQuad].transBg;
    quads[activeQuad].transDuration = quads[sourceQuad].transDuration;
    quads[activeQuad].slideshowBg = quads[sourceQuad].slideshowBg;
    quads[activeQuad].slideshowSpeed = quads[sourceQuad].slideshowSpeed;
    quads[activeQuad].slideFit = quads[sourceQuad].slideFit;
    quads[activeQuad].slideKeepAspect = quads[sourceQuad].slideKeepAspect;
    quads[activeQuad].camBg = quads[sourceQuad].camBg;
    quads[activeQuad].camWidth = quads[sourceQuad].camWidth;
    quads[activeQuad].camHeight = quads[sourceQuad].camHeight;
    quads[activeQuad].camHFlip = quads[sourceQuad].camHFlip;
    quads[activeQuad].camVFlip = quads[sourceQuad].camVFlip;
    quads[activeQuad].camMultX = quads[sourceQuad].camMultX;
    quads[activeQuad].camMultY = quads[sourceQuad].camMultY;
    quads[activeQuad].imgMultX = quads[sourceQuad].imgMultX;
    quads[activeQuad].imgMultY = quads[sourceQuad].imgMultY;
    quads[activeQuad].videoMultX = quads[sourceQuad].videoMultX;
    quads[activeQuad].videoMultY = quads[sourceQuad].videoMultY;
    quads[activeQuad].videoSpeed = quads[sourceQuad].videoSpeed;
    quads[activeQuad].videoVolume = quads[sourceQuad].videoVolume;
    quads[activeQuad].videoLoop = quads[sourceQuad].videoLoop;
    quads[activeQuad].bgColor.r = quads[sourceQuad].bgColor.r;
    quads[activeQuad].bgColor.g = quads[sourceQuad].bgColor.g;
    quads[activeQuad].bgColor.b = quads[sourceQuad].bgColor.b;
    quads[activeQuad].bgColor.a = quads[sourceQuad].bgColor.a;
    quads[activeQuad].secondColor.r = quads[sourceQuad].secondColor.r;
    quads[activeQuad].secondColor.g = quads[sourceQuad].secondColor.g;
    quads[activeQuad].secondColor.b = quads[sourceQuad].secondColor.b;
    quads[activeQuad].secondColor.a = quads[sourceQuad].secondColor.a;
    quads[activeQuad].imgColorize.r = quads[sourceQuad].imgColorize.r;
    quads[activeQuad].imgColorize.g = quads[sourceQuad].imgColorize.g;
    quads[activeQuad].imgColorize.b = quads[sourceQuad].imgColorize.b;
    quads[activeQuad].imgColorize.a = quads[sourceQuad].imgColorize.a;
    quads[activeQuad].videoColorize.r = quads[sourceQuad].videoColorize.r;
    quads[activeQuad].videoColorize.g = quads[sourceQuad].videoColorize.g;
    quads[activeQuad].videoColorize.b = quads[sourceQuad].videoColorize.b;
    quads[activeQuad].videoColorize.a = quads[sourceQuad].videoColorize.a;
    quads[activeQuad].camColorize.r = quads[sourceQuad].camColorize.r;
    quads[activeQuad].camColorize.g = quads[sourceQuad].camColorize.g;
    quads[activeQuad].camColorize.b = quads[sourceQuad].camColorize.b;
    quads[activeQuad].camColorize.a = quads[sourceQuad].camColorize.a;
    quads[activeQuad].bBlendModes = quads[sourceQuad].bBlendModes;
    quads[activeQuad].blendMode = quads[sourceQuad].blendMode;
    quads[activeQuad].edgeBlendBool = quads[sourceQuad].edgeBlendBool;
    quads[activeQuad].edgeBlendExponent = quads[sourceQuad].edgeBlendExponent;
    quads[activeQuad].edgeBlendGamma = quads[sourceQuad].edgeBlendGamma;
    quads[activeQuad].edgeBlendLuminance = quads[sourceQuad].edgeBlendLuminance;
    quads[activeQuad].edgeBlendAmountSin = quads[sourceQuad].edgeBlendAmountSin;
    quads[activeQuad].edgeBlendAmountDx = quads[sourceQuad].edgeBlendAmountDx;
    quads[activeQuad].edgeBlendAmountTop = quads[sourceQuad].edgeBlendAmountTop;
    quads[activeQuad].edgeBlendAmountBottom = quads[sourceQuad].edgeBlendAmountBottom;
    //mask stuff
    quads[activeQuad].bMask = quads[sourceQuad].bMask;
    quads[activeQuad].maskInvert = quads[sourceQuad].maskInvert;
    int nOfMaskPoints =  quads[sourceQuad].maskPoints.size();
    quads[activeQuad].maskPoints.clear();
    if (nOfMaskPoints > 0)
    {
        for(int j=0; j<nOfMaskPoints; j++)
        {
            ofPoint tempMaskPoint;
            tempMaskPoint.x = quads[sourceQuad].maskPoints[j].x;
            tempMaskPoint.y = quads[sourceQuad].maskPoints[j].y;
            quads[activeQuad].maskPoints.push_back(tempMaskPoint);
        }
    }
    quads[activeQuad].crop[0] = quads[sourceQuad].crop[0];
    quads[activeQuad].crop[1] = quads[sourceQuad].crop[1];
    quads[activeQuad].crop[2] = quads[sourceQuad].crop[2];
    quads[activeQuad].crop[3] = quads[sourceQuad].crop[3];
    quads[activeQuad].circularCrop[0] = quads[sourceQuad].circularCrop[0];
    quads[activeQuad].circularCrop[1] = quads[sourceQuad].circularCrop[1];
    quads[activeQuad].circularCrop[2] = quads[sourceQuad].circularCrop[2];
    // deform stuff
    quads[activeQuad].bDeform = quads[sourceQuad].bDeform;
    quads[activeQuad].bBezier = quads[sourceQuad].bBezier;
    quads[activeQuad].bGrid = quads[sourceQuad].bGrid;
    // bezier stuff
    for (int j = 0; j < 4; ++j)
    {
        for (int k = 0; k < 4; ++k)
        {
            quads[activeQuad].bezierPoints[j][k][0] = quads[sourceQuad].bezierPoints[j][k][0];
            quads[activeQuad].bezierPoints[j][k][1] = quads[sourceQuad].bezierPoints[j][k][1];
            quads[activeQuad].bezierPoints[j][k][2] = quads[sourceQuad].bezierPoints[j][k][2];
        }
    }
    }
}
