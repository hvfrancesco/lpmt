#include "testApp.h"
#include "ofMain.h"
#include "ofAppGlutWindow.h"

//========================================================================
int main( )
{
	
	
	
    ofAppGlutWindow* window = new ofAppGlutWindow();
    //window->setOpenGLVersion(4, 3);    // should be default value
    // window->setOpenGLVersion(3, 2);  // 
    // window->setOpenGLVersion(2, 1);  // 

	
    ofSetupOpenGL(window, 800,600, OF_WINDOW);			// <-------- setup the GL context

    // this kicks off the running of my app
    // can be OF_WINDOW or OF_FULLSCREEN
    // pass in width and height too:

    ofRunApp(new testApp());

}
