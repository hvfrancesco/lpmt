#include "testApp.h"

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
    if ( m.getAddress() == "/active/corners/0" )
    {
        // arguments are ff
        float osc_coord_x = m.getArgAsFloat( 0 );
	float osc_coord_y = m.getArgAsFloat( 1 );
        quads[activeQuad].corners[0].x = osc_coord_x;
	quads[activeQuad].corners[0].y = osc_coord_y;
    }

    if ( m.getAddress() == "/active/corners/1" )
    {
        // arguments are ff
        float osc_coord_x = m.getArgAsFloat( 0 );
	float osc_coord_y = m.getArgAsFloat( 1 );
        quads[activeQuad].corners[1].x = osc_coord_x;
	quads[activeQuad].corners[1].y = osc_coord_y;
    }

    if ( m.getAddress() == "/active/corners/2" )
    {
        // arguments are ff
        float osc_coord_x = m.getArgAsFloat( 0 );
	float osc_coord_y = m.getArgAsFloat( 1 );
        quads[activeQuad].corners[2].x = osc_coord_x;
	quads[activeQuad].corners[2].y = osc_coord_y;
    }

    if ( m.getAddress() == "/active/corners/3" )
    {
        // arguments are ff
        float osc_coord_x = m.getArgAsFloat( 0 );
	float osc_coord_y = m.getArgAsFloat( 1 );
        quads[activeQuad].corners[3].x = osc_coord_x;
	quads[activeQuad].corners[3].y = osc_coord_y;
    }


    // resync
    else if ( m.getAddress() == "/projection/resync" )
    {
        resync();
    }

    // stop
    else if ( m.getAddress() == "/projection/stop" )
    {
        stopProjection();
    }

    // start
    else if ( m.getAddress() == "/projection/start" )
    {
        startProjection();
    }

    // save
    else if ( m.getAddress() == "/projection/save" )
    {
        setXml();
        XML.saveFile("_lpmt_settings.xml");
        cout<<"saved settings to data/_lpmt_settings.xml"<<endl;
    }

    // load
    else if ( m.getAddress() == "/projection/load" )
    {
        XML.loadFile("_lpmt_settings.xml");
        getXml();
        cout<<"loaded settings from data/_lpmt_settings.xml"<<endl;
        gui.setPage((activeQuad*3)+2);
    }

    // toggle fullscreen
    else if ( m.getAddress() == "/projection/fullscreen/toggle" )
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

    else if ( m.getAddress() == "/projection/fullscreen/on" )
    {
        bFullscreen = true;
        ofSetFullscreen(true);
    }

    else if ( m.getAddress() == "/projection/fullscreen/off" )
    {
        bFullscreen = false;
        ofSetWindowShape(WINDOW_W, WINDOW_H);
        ofSetFullscreen(false);
        // figure out how to put the window in the center:
        int screenW = ofGetScreenWidth();
        int screenH = ofGetScreenHeight();
        ofSetWindowPosition(screenW/2-WINDOW_W/2, screenH/2-WINDOW_H/2);
    }

    // toggle gui
    else if ( m.getAddress() == "/projection/gui/toggle" )
    {
        gui.toggleDraw();
        bGui = !bGui;
    }

    else if ( m.getAddress() == "/projection/mode/masksetup/toggle" )
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

    else if ( m.getAddress() == "/projection/mode/masksetup/on" )
    {
        if (!bGui){
        maskSetup = true;
        for(int i = 0; i < 36; i++)
            {
                if (quads[i].initialized)
                {
                    quads[i].isMaskSetup = true;
                }
            }
        }
    }

    else if ( m.getAddress() == "/projection/mode/masksetup/off" )
    {
        if (!bGui){
        maskSetup = false;
        for(int i = 0; i < 36; i++)
            {
                if (quads[i].initialized)
                {
                    quads[i].isMaskSetup = false;
                }
            }
        }
    }

    // toggle setup
    else if ( m.getAddress() == "/projection/mode/setup/toggle" )
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

    else if ( m.getAddress() == "/projection/mode/setup/on" )
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

    else if ( m.getAddress() == "/projection/mode/setup/off" )
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

    // connects to mpe server
    else if ( m.getAddress() == "/projection/mpe/connect" )
    {
        mpeSetup();
    }


    // timeline stuff

    // use toggle
    else if ( m.getAddress() == "/projection/timeline/toggle" )
    {
        // no argument
        useTimeline = !useTimeline;
    }

    // use
    else if ( m.getAddress() == "/projection/timeline/use" )
    {
        // argument is int32
        int osc_timeline = m.getArgAsInt32( 0 );
        if(osc_timeline == 0) {useTimeline = false;}
        else if(osc_timeline == 1) {useTimeline = true;}
    }

    // timeline duration in seconds
    else if ( m.getAddress() == "/projection/timeline/duration" )
    {
        // argument is float
        float osc_timelineDurationSeconds = m.getArgAsFloat( 0 );
        if(osc_timelineDurationSeconds >= 10.0) {timelineDurationSeconds =osc_timelineDurationSeconds;}
    }


//------------------------------------------------------
// active quad stuff
//------------------------------------------------------

    // change active quad
    else if ( m.getAddress() == "/active/set" )
    {
        // argument is int32
        int osc_activequad = m.getArgAsInt32( 0 );
        if (osc_activequad <= nOfQuads-1)
        {
            quads[activeQuad].isActive = False;
            activeQuad = osc_activequad;
            quads[activeQuad].isActive = True;
            gui.setPage((activeQuad*3)+2);
        }
    }

    // on/off
    else if ( m.getAddress() == "/active/show" )
    {
        // argument is int32
        int osc_quad_isOn = m.getArgAsInt32( 0 );
        if(osc_quad_isOn == 0) {quads[activeQuad].isOn = false;}
        else if(osc_quad_isOn == 1) {quads[activeQuad].isOn = true;}
    }

    // use timeline color
    else if ( m.getAddress() == "/active/timeline/color" )
    {
        // argument is int32
        int osc_quad_bTimelineColor = m.getArgAsInt32( 0 );
        if(osc_quad_bTimelineColor == 0) {quads[activeQuad].bTimelineColor = false;}
        else if(osc_quad_bTimelineColor == 1) {quads[activeQuad].bTimelineColor = true;}
    }

    // use timeline color
    else if ( m.getAddress() == "/active/timeline/alpha" )
    {
        // argument is int32
        int osc_quad_bTimelineAlpha = m.getArgAsInt32( 0 );
        if(osc_quad_bTimelineAlpha == 0) {quads[activeQuad].bTimelineAlpha = false;}
        else if(osc_quad_bTimelineAlpha == 1) {quads[activeQuad].bTimelineAlpha = true;}
    }

    // use timeline for slides
    else if ( m.getAddress() == "/active/timeline/slides" )
    {
        // argument is int32
        int osc_quad_bTimelineSlideChange = m.getArgAsInt32( 0 );
        if(osc_quad_bTimelineSlideChange == 0) {quads[activeQuad].bTimelineSlideChange = false;}
        else if(osc_quad_bTimelineSlideChange == 1) {quads[activeQuad].bTimelineSlideChange = true;}
    }

    // img stuff on active quad
    else if ( m.getAddress() == "/active/img" )
    {
        quads[activeQuad].imgBg = !quads[activeQuad].imgBg;
    }

    // img on/off
    else if ( m.getAddress() == "/active/img/show" )
    {
        // argument is int32
        int osc_quad_imgBg = m.getArgAsInt32( 0 );
        if(osc_quad_imgBg == 0) {quads[activeQuad].imgBg = false;}
        else if(osc_quad_imgBg == 1) {quads[activeQuad].imgBg = true;}
    }

    // img load
    else if ( m.getAddress() == "/active/img/load" )
    {
        // no argument
        openImageFile();
    }

    // img HFlip
    else if ( m.getAddress() == "/active/img/hmirror" )
    {
        // argument is int32
        int osc_quad_imgHFlip = m.getArgAsInt32( 0 );
        if(osc_quad_imgHFlip == 0) {quads[activeQuad].imgHFlip = false;}
        else if(osc_quad_imgHFlip == 1) {quads[activeQuad].imgHFlip = true;}
    }

    // img VFlip
    else if ( m.getAddress() == "/active/img/vmirror" )
    {
        // argument is int32
        int osc_quad_imgVFlip = m.getArgAsInt32( 0 );
        if(osc_quad_imgVFlip == 0) {quads[activeQuad].imgVFlip = false;}
        else if(osc_quad_imgVFlip == 1) {quads[activeQuad].imgVFlip = true;}
    }

    else if ( m.getAddress() == "/active/img/color" )
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

    else if ( m.getAddress() == "/active/img/color/1" )
    {
        // arguments are f
        float img_color_r = m.getArgAsFloat( 0 );
        quads[activeQuad].imgColorize.r = img_color_r;
    }

    else if ( m.getAddress() == "/active/img/color/2" )
    {
        // arguments are f
        float img_color_g = m.getArgAsFloat( 0 );
        quads[activeQuad].imgColorize.g = img_color_g;
    }

    else if ( m.getAddress() == "/active/img/color/3" )
    {
        // arguments are f
        float img_color_b = m.getArgAsFloat( 0 );
        quads[activeQuad].imgColorize.b = img_color_b;
    }

    else if ( m.getAddress() == "/active/img/color/4" )
    {
        // arguments are f
        float img_color_a = m.getArgAsFloat( 0 );
        quads[activeQuad].imgColorize.a = img_color_a;
    }

    else if ( m.getAddress() == "/active/img/mult/x" )
    {
        // arguments are f
        float img_mult_x = m.getArgAsFloat( 0 );
        quads[activeQuad].imgMultX = img_mult_x;
    }

    else if ( m.getAddress() == "/active/img/mult/y" )
    {
        // arguments are f
        float img_mult_y = m.getArgAsFloat( 0 );
        quads[activeQuad].imgMultY = img_mult_y;
    }


    // blendModes on/off
    else if ( m.getAddress() == "/active/blendmodes/show" )
    {
        // argument is int32
        int osc_quad_bBlendModes = m.getArgAsInt32( 0 );
        if(osc_quad_bBlendModes == 0) {quads[activeQuad].bBlendModes = false;}
        else if(osc_quad_bBlendModes == 1) {quads[activeQuad].bBlendModes = true;}
    }

    // blendModes mode
    else if ( m.getAddress() == "/active/blendmodes/mode" )
    {
        // argument is int32
        int osc_quad_blendMode = m.getArgAsInt32( 0 );
        if(osc_quad_blendMode < 4) {quads[activeQuad].blendMode = osc_quad_blendMode;}
    }

    // solid color stuff
    else if ( m.getAddress() == "/active/solid" )
    {
        quads[activeQuad].colorBg = !quads[activeQuad].colorBg;
    }

    // solid on/off
    else if ( m.getAddress() == "/active/solid/show" )
    {
        // argument is int32
        int osc_quad_colorBg = m.getArgAsInt32( 0 );
        if(osc_quad_colorBg == 0) {quads[activeQuad].colorBg = false;}
        else if(osc_quad_colorBg == 1) {quads[activeQuad].colorBg = true;}
    }

    else if ( m.getAddress() == "/active/solid/color" )
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

    else if ( m.getAddress() == "/active/solid/color/1" )
    {
        // arguments are f
        float solid_color_r = m.getArgAsFloat( 0 );
        quads[activeQuad].bgColor.r = solid_color_r;
    }

    else if ( m.getAddress() == "/active/solid/color/2" )
    {
        // arguments are f
        float solid_color_g = m.getArgAsFloat( 0 );
        quads[activeQuad].bgColor.g = solid_color_g;
    }

    else if ( m.getAddress() == "/active/solid/color/3" )
    {
        // arguments are f
        float solid_color_b = m.getArgAsFloat( 0 );
        quads[activeQuad].bgColor.b = solid_color_b;
    }

    else if ( m.getAddress() == "/active/solid/color/4" )
    {
        // arguments are f
        float solid_color_a = m.getArgAsFloat( 0 );
        quads[activeQuad].bgColor.a = solid_color_a;
    }

    else if ( m.getAddress() == "/active/solid/trans" )
    {
        quads[activeQuad].transBg = !quads[activeQuad].transBg;
    }

    // trans on/off
    else if ( m.getAddress() == "/active/solid/trans/show" )
    {
        // argument is int32
        int osc_quad_transBg = m.getArgAsInt32( 0 );
        if(osc_quad_transBg == 0) {quads[activeQuad].transBg = false;}
        else if(osc_quad_transBg == 1) {quads[activeQuad].transBg = true;}
    }

    else if ( m.getAddress() == "/active/solid/trans/color" )
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

    else if ( m.getAddress() == "/active/solid/trans/color/1" )
    {
        // arguments are f
        float trans_color_r = m.getArgAsFloat( 0 );
        quads[activeQuad].secondColor.r = trans_color_r;
    }

    else if ( m.getAddress() == "/active/solid/trans/color/2" )
    {
        // arguments are f
        float trans_color_g = m.getArgAsFloat( 0 );
        quads[activeQuad].secondColor.g = trans_color_g;
    }

    else if ( m.getAddress() == "/active/solid/trans/color/3" )
    {
        // arguments are f
        float trans_color_b = m.getArgAsFloat( 0 );
        quads[activeQuad].secondColor.b = trans_color_b;
    }

    else if ( m.getAddress() == "/active/solid/trans/color/4" )
    {
        // arguments are f
        float trans_color_a = m.getArgAsFloat( 0 );
        quads[activeQuad].secondColor.a = trans_color_a;
    }

    else if ( m.getAddress() == "/active/solid/trans/duration" )
    {
        // arguments are f
        float trans_duration = m.getArgAsFloat( 0 );
        quads[activeQuad].transDuration = trans_duration;
    }


    // mask stuff on active quad
    else if ( m.getAddress() == "/active/mask" )
    {
        quads[activeQuad].bMask = !quads[activeQuad].bMask;
    }

    // mask on/off
    else if ( m.getAddress() == "/active/mask/show" )
    {
        // argument is int32
        int osc_quad_bMask = m.getArgAsInt32( 0 );
        if(osc_quad_bMask == 0) {quads[activeQuad].bMask = false;}
        else if(osc_quad_bMask == 1) {quads[activeQuad].bMask = true;}
    }

    // mask invert
    else if ( m.getAddress() == "/active/mask/invert" )
    {
        // argument is int32
        int osc_quad_maskInvert = m.getArgAsInt32( 0 );
        if(osc_quad_maskInvert == 0) {quads[activeQuad].maskInvert = false;}
        else if(osc_quad_maskInvert == 1) {quads[activeQuad].maskInvert = true;}
    }

    // deform stuff

    // deform on/off
    else if ( m.getAddress() == "/active/deform/show" )
    {
        // argument is int32
        int osc_quad_bDeform = m.getArgAsInt32( 0 );
        if(osc_quad_bDeform == 0) {quads[activeQuad].bDeform = false;}
        else if(osc_quad_bDeform == 1) {quads[activeQuad].bDeform = true;}
    }

    // deform bezier
    else if ( m.getAddress() == "/active/deform/bezier" )
    {
        // argument is int32
        int osc_quad_bBezier = m.getArgAsInt32( 0 );
        if(osc_quad_bBezier == 0) {quads[activeQuad].bBezier = false;}
        else if(osc_quad_bBezier == 1) {quads[activeQuad].bBezier = true;}
    }

    else if ( m.getAddress() == "/active/deform/bezier/spherize/light" )
    {
        // no argument
        quadBezierSpherize(activeQuad);
    }

    else if ( m.getAddress() == "/active/deform/bezier/spherize/strong" )
    {
        // no argument
        quadBezierSpherizeStrong(activeQuad);
    }

   else if ( m.getAddress() == "/active/deform/bezier/reset" )
    {
        // no argument
        quadBezierReset(activeQuad);
    }

    // deform grid
    else if ( m.getAddress() == "/active/deform/grid" )
    {
        // argument is int32
        int osc_quad_bGrid = m.getArgAsInt32( 0 );
        if(osc_quad_bGrid == 0) {quads[activeQuad].bGrid = false;}
        else if(osc_quad_bGrid == 1) {quads[activeQuad].bGrid = true;}
    }

    // deform grid rows
    else if ( m.getAddress() == "/active/deform/grid/rows" )
    {
        // argument is int32
        int osc_quad_gridRows = m.getArgAsInt32( 0 );
        if(osc_quad_gridRows >= 2 && osc_quad_gridRows <= 15) {quads[activeQuad].gridRows = osc_quad_gridRows;}
    }

    // deform grid columns
    else if ( m.getAddress() == "/active/deform/grid/columns" )
    {
        // argument is int32
        int osc_quad_gridColumns = m.getArgAsInt32( 0 );
        if(osc_quad_gridColumns >= 2 && osc_quad_gridColumns <= 20) {quads[activeQuad].gridColumns = osc_quad_gridColumns;}
    }

    // edge-blend stuff
    // edge-blend on/off
    else if ( m.getAddress() == "/active/edgeblend/show" )
    {
        // argument is int32
        int osc_quad_edgeBlendBool = m.getArgAsInt32( 0 );
        if(osc_quad_edgeBlendBool == 0) {quads[activeQuad].edgeBlendBool = false;}
        else if(osc_quad_edgeBlendBool == 1) {quads[activeQuad].edgeBlendBool = true;}
    }

    else if ( m.getAddress() == "/active/edgeblend/power" )
    {
        // argument is float
        float osc_quad_edgeBlendExponent = m.getArgAsFloat( 0 );
        quads[activeQuad].edgeBlendExponent = osc_quad_edgeBlendExponent;
    }

    else if ( m.getAddress() == "/active/edgeblend/gamma" )
    {
        // argument is float
        float osc_quad_edgeBlendGamma = m.getArgAsFloat( 0 );
        quads[activeQuad].edgeBlendGamma = osc_quad_edgeBlendGamma;
    }

    else if ( m.getAddress() == "/active/edgeblend/luminance" )
    {
        // argument is float
        float osc_quad_edgeBlendLuminance = m.getArgAsFloat( 0 );
        quads[activeQuad].edgeBlendLuminance = osc_quad_edgeBlendLuminance;
    }

    else if ( m.getAddress() == "/active/edgeblend/amount/left" )
    {
        // argument is float
        float osc_quad_edgeBlendAmountSin = m.getArgAsFloat( 0 );
        quads[activeQuad].edgeBlendAmountSin = osc_quad_edgeBlendAmountSin;
    }

    else if ( m.getAddress() == "/active/edgeblend/amount/right" )
    {
        // argument is float
        float osc_quad_edgeBlendAmountDx = m.getArgAsFloat( 0 );
        quads[activeQuad].edgeBlendAmountDx = osc_quad_edgeBlendAmountDx;
    }

    else if ( m.getAddress() == "/active/edgeblend/amount/top" )
    {
        // argument is float
        float osc_quad_edgeBlendAmountTop = m.getArgAsFloat( 0 );
        quads[activeQuad].edgeBlendAmountTop = osc_quad_edgeBlendAmountTop;
    }

    else if ( m.getAddress() == "/active/edgeblend/amount/bottom" )
    {
        // argument is float
        float osc_quad_edgeBlendAmountBottom = m.getArgAsFloat( 0 );
        quads[activeQuad].edgeBlendAmountBottom = osc_quad_edgeBlendAmountBottom;
    }

    else if ( m.getAddress() == "/active/edgeblend/amount" )
    {
        // argument is ffff
        float osc_quad_edgeBlendAmountTop = m.getArgAsFloat( 0 );
        float osc_quad_edgeBlendAmountDx = m.getArgAsFloat( 1 );
        float osc_quad_edgeBlendAmountBottom = m.getArgAsFloat( 2 );
        float osc_quad_edgeBlendAmountSin = m.getArgAsFloat( 3 );
        quads[activeQuad].edgeBlendAmountTop = osc_quad_edgeBlendAmountTop;
        quads[activeQuad].edgeBlendAmountDx = osc_quad_edgeBlendAmountDx;
        quads[activeQuad].edgeBlendAmountBottom = osc_quad_edgeBlendAmountBottom;
        quads[activeQuad].edgeBlendAmountSin = osc_quad_edgeBlendAmountSin;
    }

    // content displacement
    // displacement X
    else if ( m.getAddress() == "/active/placement/x" )
    {
        // argument is int32
        int osc_quad_quadDispX = m.getArgAsInt32( 0 );
        quads[activeQuad].quadDispX = osc_quad_quadDispX;
    }

    // displacement Y
    else if ( m.getAddress() == "/active/placement/y" )
    {
        // argument is int32
        int osc_quad_quadDispY = m.getArgAsInt32( 0 );
        quads[activeQuad].quadDispY = osc_quad_quadDispY;
    }

    // displacement XY
    else if ( m.getAddress() == "/active/placement" )
    {
        // argument is int32 int32
        int osc_quad_quadDispX = m.getArgAsInt32( 0 );
        int osc_quad_quadDispY = m.getArgAsInt32( 1 );
        quads[activeQuad].quadDispX = osc_quad_quadDispX;
        quads[activeQuad].quadDispY = osc_quad_quadDispY;
    }

    // displacement W
    else if ( m.getAddress() == "/active/placement/w" )
    {
        // argument is int32
        int osc_quad_quadW = m.getArgAsInt32( 0 );
        quads[activeQuad].quadW = osc_quad_quadW;
    }

    // displacement H
    else if ( m.getAddress() == "/active/placement/h" )
    {
        // argument is int32
        int osc_quad_quadH = m.getArgAsInt32( 0 );
        quads[activeQuad].quadH = osc_quad_quadH;
    }

    // displacement WH
    else if ( m.getAddress() == "/active/placement/dimensions" )
    {
        // argument is int32 int32
        int osc_quad_quadW = m.getArgAsInt32( 0 );
        int osc_quad_quadH = m.getArgAsInt32( 1 );
        quads[activeQuad].quadW = osc_quad_quadW;
        quads[activeQuad].quadH = osc_quad_quadH;
    }

    // displacement reset
    else if ( m.getAddress() == "/active/placement/reset" )
    {
        // no argument
        quadDimensionsReset(activeQuad);
        quadPlacementReset(activeQuad);
    }

    // video stuff on active quad
    else if ( m.getAddress() == "/active/video" )
    {
        quads[activeQuad].videoBg = !quads[activeQuad].videoBg;
    }

    // video on/off
    else if ( m.getAddress() == "/active/video/show" )
    {
        // argument is int32
        int osc_quad_videoBg = m.getArgAsInt32( 0 );
        if(osc_quad_videoBg == 0) {quads[activeQuad].videoBg = false;}
        else if(osc_quad_videoBg == 1) {quads[activeQuad].videoBg = true;}
    }

    // video load
    else if ( m.getAddress() == "/active/video/load" )
    {
        // no argument
        openVideoFile();
    }

    // video HFlip
    else if ( m.getAddress() == "/active/video/hmirror" )
    {
        // argument is int32
        int osc_quad_videoHFlip = m.getArgAsInt32( 0 );
        if(osc_quad_videoHFlip == 0) {quads[activeQuad].videoHFlip = false;}
        else if(osc_quad_videoHFlip == 1) {quads[activeQuad].videoHFlip = true;}
    }

    // video VFlip
    else if ( m.getAddress() == "/active/video/vmirror" )
    {
        // argument is int32
        int osc_quad_videoVFlip = m.getArgAsInt32( 0 );
        if(osc_quad_videoVFlip == 0) {quads[activeQuad].videoVFlip = false;}
        else if(osc_quad_videoVFlip == 1) {quads[activeQuad].videoVFlip = true;}
    }



    else if ( m.getAddress() == "/active/video/color" )
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

    else if ( m.getAddress() == "/active/video/color/1" )
    {
        // arguments are f
        float video_color_r = m.getArgAsFloat( 0 );
        quads[activeQuad].videoColorize.r = video_color_r;
    }

    else if ( m.getAddress() == "/active/video/color/2" )
    {
        // arguments are f
        float video_color_g = m.getArgAsFloat( 0 );
        quads[activeQuad].videoColorize.g = video_color_g;
    }

    else if ( m.getAddress() == "/active/video/color/3" )
    {
        // arguments are f
        float video_color_b = m.getArgAsFloat( 0 );
        quads[activeQuad].videoColorize.b = video_color_b;
    }

    else if ( m.getAddress() == "/active/video/color/4" )
    {
        // arguments are f
        float video_color_a = m.getArgAsFloat( 0 );
        quads[activeQuad].videoColorize.a = video_color_a;
    }

    else if ( m.getAddress() == "/active/video/mult/x" )
    {
        // arguments are f
        float video_mult_x = m.getArgAsFloat( 0 );
        quads[activeQuad].videoMultX = video_mult_x;
    }

    else if ( m.getAddress() == "/active/video/mult/y" )
    {
        // arguments are f
        float video_mult_y = m.getArgAsFloat( 0 );
        quads[activeQuad].videoMultY = video_mult_y;
    }

    else if ( m.getAddress() == "/active/video/speed" )
    {
        // arguments are f
        float video_speed = m.getArgAsFloat( 0 );
        quads[activeQuad].videoSpeed = video_speed;
    }

    else if ( m.getAddress() == "/active/video/volume" )
    {
        // arguments are i
        int video_volume = m.getArgAsInt32( 0 );
        quads[activeQuad].videoVolume = video_volume;
    }

    // video loop
    else if ( m.getAddress() == "/active/video/loop" )
    {
        // argument is int32
        int osc_quad_videoLoop = m.getArgAsInt32( 0 );
        if(osc_quad_videoLoop == 0) {quads[activeQuad].videoLoop = false;}
        else if(osc_quad_videoLoop == 1) {quads[activeQuad].videoLoop = true;}
    }

    // video greenscreen
    else if ( m.getAddress() == "/active/video/greenscreen" )
    {
        // argument is int32
        int osc_quad_videoGreenscreen = m.getArgAsInt32( 0 );
        if(osc_quad_videoGreenscreen == 0) {quads[activeQuad].videoGreenscreen = false;}
        else if(osc_quad_videoGreenscreen == 1) {quads[activeQuad].videoGreenscreen = true;}
    }

    // camera stuff

    else if ( m.getAddress() == "/active/cam" )
    {
        quads[activeQuad].camBg = !quads[activeQuad].camBg;
    }

    // video on/off
    else if ( m.getAddress() == "/active/cam/show" )
    {
        // argument is int32
        int osc_quad_camBg = m.getArgAsInt32( 0 );
        if(osc_quad_camBg == 0) {quads[activeQuad].camBg = false;}
        else if(osc_quad_camBg == 1) {quads[activeQuad].camBg = true;}
    }

    // video HFlip
    else if ( m.getAddress() == "/active/cam/hmirror" )
    {
        // argument is int32
        int osc_quad_camHFlip = m.getArgAsInt32( 0 );
        if(osc_quad_camHFlip == 0) {quads[activeQuad].camHFlip = false;}
        else if(osc_quad_camHFlip == 1) {quads[activeQuad].camHFlip = true;}
    }

    // video VFlip
    else if ( m.getAddress() == "/active/cam/vmirror" )
    {
        // argument is int32
        int osc_quad_camVFlip = m.getArgAsInt32( 0 );
        if(osc_quad_camVFlip == 0) {quads[activeQuad].camVFlip = false;}
        else if(osc_quad_camVFlip == 1) {quads[activeQuad].camVFlip = true;}
    }


    else if ( m.getAddress() == "/active/cam/color" )
    {
        // arguments are ffff
        float cam_color_r = m.getArgAsFloat( 0 );
        float cam_color_g = m.getArgAsFloat( 1 );
        float cam_color_b = m.getArgAsFloat( 2 );
        float cam_color_a = m.getArgAsFloat( 3 );
        quads[activeQuad].camColorize.r = cam_color_r;
        quads[activeQuad].camColorize.g = cam_color_g;
        quads[activeQuad].camColorize.b = cam_color_b;
        quads[activeQuad].camColorize.a = cam_color_a;
    }

    else if ( m.getAddress() == "/active/cam/color/1" )
    {
        // arguments are f
        float cam_color_r = m.getArgAsFloat( 0 );
        quads[activeQuad].camColorize.r = cam_color_r;
    }

    else if ( m.getAddress() == "/active/cam/color/2" )
    {
        // arguments are f
        float cam_color_g = m.getArgAsFloat( 0 );
        quads[activeQuad].camColorize.g = cam_color_g;
    }

    else if ( m.getAddress() == "/active/cam/color/3" )
    {
        // arguments are f
        float cam_color_b = m.getArgAsFloat( 0 );
        quads[activeQuad].camColorize.b = cam_color_b;
    }

    else if ( m.getAddress() == "/active/cam/color/4" )
    {
        // arguments are f
        float cam_color_a = m.getArgAsFloat( 0 );
        quads[activeQuad].camColorize.a = cam_color_a;
    }

    else if ( m.getAddress() == "/active/cam/mult/x" )
    {
        // arguments are f
        float cam_mult_x = m.getArgAsFloat( 0 );
        quads[activeQuad].camMultX = cam_mult_x;
    }

    else if ( m.getAddress() == "/active/cam/mult/y" )
    {
        // arguments are f
        float cam_mult_y = m.getArgAsFloat( 0 );
        quads[activeQuad].camMultY = cam_mult_y;
    }


    // cam greenscreen
    else if ( m.getAddress() == "/active/cam/greenscreen" )
    {
        // argument is int32
        int osc_quad_camGreenscreen = m.getArgAsInt32( 0 );
        if(osc_quad_camGreenscreen == 0) {quads[activeQuad].camGreenscreen = false;}
        else if(osc_quad_camGreenscreen == 1) {quads[activeQuad].camGreenscreen = true;}
    }

    // greenscreen stuff

    // greenscreen threshold
    else if ( m.getAddress() == "/active/greenscreen/threshold" )
    {
        // argument is int32
        int osc_quad_thresholdGreenscreen = m.getArgAsInt32( 0 );
        quads[activeQuad].thresholdGreenscreen = osc_quad_thresholdGreenscreen;
    }

    else if ( m.getAddress() == "/active/greenscreen/color" )
    {
        // arguments are ffff
        float greenscreen_color_r = m.getArgAsFloat( 0 );
        float greenscreen_color_g = m.getArgAsFloat( 1 );
        float greenscreen_color_b = m.getArgAsFloat( 2 );
        float greenscreen_color_a = m.getArgAsFloat( 3 );
        quads[activeQuad].colorGreenscreen.r = greenscreen_color_r;
        quads[activeQuad].colorGreenscreen.g = greenscreen_color_g;
        quads[activeQuad].colorGreenscreen.b = greenscreen_color_b;
        quads[activeQuad].colorGreenscreen.a = greenscreen_color_a;
    }

    else if ( m.getAddress() == "/active/greenscreen/color/1" )
    {
        // arguments are f
        float greenscreen_color_r = m.getArgAsFloat( 0 );
        quads[activeQuad].colorGreenscreen.r = greenscreen_color_r;
    }

    else if ( m.getAddress() == "/active/greenscreen/color/2" )
    {
        // arguments are f
        float greenscreen_color_g = m.getArgAsFloat( 0 );
        quads[activeQuad].colorGreenscreen.g = greenscreen_color_g;
    }

    else if ( m.getAddress() == "/active/greenscreen/color/3" )
    {
        // arguments are f
        float greenscreen_color_b = m.getArgAsFloat( 0 );
        quads[activeQuad].colorGreenscreen.b = greenscreen_color_b;
    }

    else if ( m.getAddress() == "/active/greenscreen/color/4" )
    {
        // arguments are f
        float greenscreen_color_a = m.getArgAsFloat( 0 );
        quads[activeQuad].colorGreenscreen.a = greenscreen_color_a;
    }

    // slideshow stuff

    else if ( m.getAddress() == "/active/slideshow" )
    {
        quads[activeQuad].slideshowBg = !quads[activeQuad].slideshowBg;
    }

    // slideshow on/off
    else if ( m.getAddress() == "/active/slideshow/show" )
    {
        // argument is int32
        int osc_quad_slideshowBg = m.getArgAsInt32( 0 );
        if(osc_quad_slideshowBg == 0) {quads[activeQuad].slideshowBg = false;}
        else if(osc_quad_slideshowBg == 1) {quads[activeQuad].slideshowBg = true;}
    }

    else if ( m.getAddress() == "/active/slideshow/folder" )
    {
        // argument is int32
        int osc_quad_bgSlideshow = m.getArgAsInt32( 0 );
        if(osc_quad_bgSlideshow <= slideshowFolders.size()) {quads[activeQuad].bgSlideshow = osc_quad_bgSlideshow;}
    }

    else if ( m.getAddress() == "/active/slideshow/fit" )
    {
        // argument is int32
        int osc_quad_slideFit = m.getArgAsInt32( 0 );
        if(osc_quad_slideFit == 0) {quads[activeQuad].slideFit = false;}
        else if(osc_quad_slideFit == 1) {quads[activeQuad].slideFit = true;}
    }

    else if ( m.getAddress() == "/active/slideshow/keep_aspect" )
    {
        // argument is int32
        int osc_quad_slideKeepAspect = m.getArgAsInt32( 0 );
        if(osc_quad_slideKeepAspect == 0) {quads[activeQuad].slideKeepAspect = false;}
        else if(osc_quad_slideKeepAspect == 1) {quads[activeQuad].slideKeepAspect = true;}
    }

    else if ( m.getAddress() == "/active/slideshow/duration" )
    {
        // arguments are f
        float osc_quad_slideshowSpeed = m.getArgAsFloat( 0 );
        quads[activeQuad].slideshowSpeed = osc_quad_slideshowSpeed;
    }


    // kinect stuff
    else if ( m.getAddress() == "/active/kinect" )
    {
        quads[activeQuad].kinectBg = !quads[activeQuad].kinectBg;
    }

    // slideshow on/off
    else if ( m.getAddress() == "/active/kinect/show" )
    {
        // argument is int32
        int osc_quad_kinectBg = m.getArgAsInt32( 0 );
        if(osc_quad_kinectBg == 0) {quads[activeQuad].kinectBg = false;}
        else if(osc_quad_kinectBg == 1) {quads[activeQuad].kinectBg = true;}
    }

    else if ( m.getAddress() == "/active/kinect/open" )
    {
        // no argument
        kinect.kinect.open();
    }

    else if ( m.getAddress() == "/active/kinect/close" )
    {
        // no argument
        kinect.kinect.setCameraTiltAngle(0);
        kinect.kinect.close();
    }

    // kinect image
    else if ( m.getAddress() == "/active/kinect/show/image" )
    {
        // argument is int32
        int osc_quad_kinectImg = m.getArgAsInt32( 0 );
        if(osc_quad_kinectImg == 0) {quads[activeQuad].kinectImg = false;}
        else if(osc_quad_kinectImg == 1) {quads[activeQuad].kinectImg = true;}
    }

    // kinect grayscale image
    else if ( m.getAddress() == "/active/kinect/show/grayscale" )
    {
        // argument is int32
        int osc_quad_getKinectGrayImage = m.getArgAsInt32( 0 );
        if(osc_quad_getKinectGrayImage == 0) {quads[activeQuad].getKinectGrayImage = false;}
        else if(osc_quad_getKinectGrayImage == 1) {quads[activeQuad].getKinectGrayImage = true;}
    }

    else if ( m.getAddress() == "/active/kinect/mask" )
    {
        // argument is int32
        int osc_quad_kinectMask = m.getArgAsInt32( 0 );
        if(osc_quad_kinectMask == 0) {quads[activeQuad].kinectMask = false;}
        else if(osc_quad_kinectMask == 1) {quads[activeQuad].kinectMask = true;}
    }


    else if ( m.getAddress() == "/active/kinect/mult/x" )
    {
        // arguments are f
        float kinect_mult_x = m.getArgAsFloat( 0 );
        quads[activeQuad].kinectMultX = kinect_mult_x;
    }

    else if ( m.getAddress() == "/active/kinect/mult/y" )
    {
        // arguments are f
        float kinect_mult_y = m.getArgAsFloat( 0 );
        quads[activeQuad].kinectMultY = kinect_mult_y;
    }

    else if ( m.getAddress() == "/active/kinect/scale/x" )
    {
        // arguments are f
        float kinect_mult_x = m.getArgAsFloat( 0 );
        quads[activeQuad].kinectMultX = kinect_mult_x;
    }

    else if ( m.getAddress() == "/active/kinect/scale/y" )
    {
        // arguments are f
        float kinect_mult_y = m.getArgAsFloat( 0 );
        quads[activeQuad].kinectMultY = kinect_mult_y;
    }


    else if ( m.getAddress() == "/active/kinect/threshold/near" )
    {
        // arguments are int32
        int osc_quad_nearDepthTh = m.getArgAsInt32( 0 );
        quads[activeQuad].nearDepthTh = osc_quad_nearDepthTh;
    }

    else if ( m.getAddress() == "/active/kinect/threshold/far" )
    {
        // arguments are int32
        int osc_quad_farDepthTh = m.getArgAsInt32( 0 );
        quads[activeQuad].farDepthTh = osc_quad_farDepthTh;
    }

    else if ( m.getAddress() == "/active/kinect/angle" )
    {
        // argument is int32
        int osc_quad_kinectAngle = m.getArgAsInt32( 0 );
        if(osc_quad_kinectAngle >= -30 && osc_quad_kinectAngle <= 30) {kinect.kinectAngle = osc_quad_kinectAngle;}
    }

    else if ( m.getAddress() == "/active/kinect/blur" )
    {
        // argument is int32
        int osc_quad_kinectBlur = m.getArgAsInt32( 0 );
        if(osc_quad_kinectBlur >= 0 && osc_quad_kinectBlur <= 10) {quads[activeQuad].kinectBlur = osc_quad_kinectBlur;}
    }

    else if ( m.getAddress() == "/active/kinect/contour" )
    {
        // argument is int32
        int osc_quad_getKinectContours = m.getArgAsInt32( 0 );
        if(osc_quad_getKinectContours == 0) {quads[activeQuad].getKinectContours = false;}
        else if(osc_quad_getKinectContours == 1) {quads[activeQuad].getKinectContours = true;}
    }

    else if ( m.getAddress() == "/active/kinect/contour/curves" )
    {
        // argument is int32
        int osc_quad_kinectContourCurved = m.getArgAsInt32( 0 );
        if(osc_quad_kinectContourCurved == 0) {quads[activeQuad].kinectContourCurved = false;}
        else if(osc_quad_kinectContourCurved == 1) {quads[activeQuad].kinectContourCurved = true;}
    }

    else if ( m.getAddress() == "/active/kinect/contour/smooth" )
    {
        // argument is int32
        int osc_quad_kinectContourSmooth = m.getArgAsInt32( 0 );
        if(osc_quad_kinectContourSmooth >= 0 && osc_quad_kinectContourSmooth <= 20) {quads[activeQuad].kinectContourSmooth = osc_quad_kinectContourSmooth;}
    }

    else if ( m.getAddress() == "/active/kinect/contour/simplify" )
    {
        // argument is f
        float osc_quad_kinectContourSimplify = m.getArgAsFloat( 0 );
        quads[activeQuad].kinectContourSimplify = osc_quad_kinectContourSimplify;
    }

    else if ( m.getAddress() == "/active/kinect/contour/area/min" )
    {
        // argument is f
        float osc_quad_kinectContourMin = m.getArgAsFloat( 0 );
        quads[activeQuad].kinectContourMin = osc_quad_kinectContourMin;
    }

    else if ( m.getAddress() == "/active/kinect/contour/area/max" )
    {
        // argument is f
        float osc_quad_kinectContourMax = m.getArgAsFloat( 0 );
        quads[activeQuad].kinectContourMax = osc_quad_kinectContourMax;
    }

    else if ( m.getAddress() == "/active/kinect/contour/area" )
    {
        // argument is f f
        float osc_quad_kinectContourMin = m.getArgAsFloat( 0 );
        float osc_quad_kinectContourMax = m.getArgAsFloat( 1 );
        quads[activeQuad].kinectContourMin = osc_quad_kinectContourMin;
        quads[activeQuad].kinectContourMax = osc_quad_kinectContourMax;
    }

    else if ( m.getAddress() == "/active/kinect/color" )
    {
        // arguments are ffff
        float kinect_color_r = m.getArgAsFloat( 0 );
        float kinect_color_g = m.getArgAsFloat( 1 );
        float kinect_color_b = m.getArgAsFloat( 2 );
        float kinect_color_a = m.getArgAsFloat( 3 );
        quads[activeQuad].kinectColorize.r = kinect_color_r;
        quads[activeQuad].kinectColorize.g = kinect_color_g;
        quads[activeQuad].kinectColorize.b = kinect_color_b;
        quads[activeQuad].kinectColorize.a = kinect_color_a;
    }

    else if ( m.getAddress() == "/active/kinect/color/1" )
    {
        // arguments are f
        float kinect_color_r = m.getArgAsFloat( 0 );
        quads[activeQuad].kinectColorize.r = kinect_color_r;
    }

    else if ( m.getAddress() == "/active/kinect/color/2" )
    {
        // arguments are f
        float kinect_color_g = m.getArgAsFloat( 0 );
        quads[activeQuad].kinectColorize.g = kinect_color_g;
    }

    else if ( m.getAddress() == "/active/kinect/color/3" )
    {
        // arguments are f
        float kinect_color_b = m.getArgAsFloat( 0 );
        quads[activeQuad].kinectColorize.b = kinect_color_b;
    }

    else if ( m.getAddress() == "/active/kinect/color/4" )
    {
        // arguments are f
        float kinect_color_a = m.getArgAsFloat( 0 );
        quads[activeQuad].kinectColorize.a = kinect_color_a;
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
