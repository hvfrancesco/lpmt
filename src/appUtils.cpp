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
