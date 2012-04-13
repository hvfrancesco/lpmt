#include "testApp.h"

//-----------------------------------------------------------
void testApp::openImageFile()
{
    ofFileDialogResult dialog_result = ofSystemLoadDialog("load image file", false);
    if(dialog_result.bSuccess)
    {
        quads[activeQuad].loadImageFromFile(dialog_result.getName(), dialog_result.getPath());
    }
}

//-----------------------------------------------------------
void testApp::openVideoFile()
{
    ofFileDialogResult dialog_result = ofSystemLoadDialog("load video file", false);
    if(dialog_result.bSuccess)
    {
        quads[activeQuad].loadVideoFromFile(dialog_result.getName(), dialog_result.getPath());
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
    if(useTimeline)
    {
        timeline.setCurrentTime(0.0);
    }

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
    if(useTimeline)
    {
        timeline.enable();
        timeline.play();
    }
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
    if(useTimeline)
    {
        timeline.stop();
        timeline.hide();
        timeline.disable();
    }
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
