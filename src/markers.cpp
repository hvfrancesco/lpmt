#include "quad.h"

// Markers -----------------------------------------------------

// Bezier markers ----------------------------------------------
void quad::drawBezierMarkers()
{
    ofSetColor(220,200,0,255);
    ofSetLineWidth(1.5);
    for(unsigned int i = 0; i < 4; i++)
    {
        for(unsigned int j = 0; j < 4; j++)
        {
            ofVec3f punto;
            punto.x = bezierCtrlPoints[i][j][0];
            punto.y = bezierCtrlPoints[i][j][1];
            punto.z = bezierCtrlPoints[i][j][2];
            punto = findWarpedPoint(dst, src, punto);

            if(bHighlightCtrlPoint && highlightedCtrlPointRow == i && highlightedCtrlPointCol == j)
            {
                ofFill();
            }
            ofCircle(punto.x, punto.y, 3.6);
            ofNoFill();
        }
    }
    ofSetLineWidth(1.2);
    ofVec3f puntoA;
    ofVec3f puntoB;
    //
    puntoA.x = bezierCtrlPoints[0][0][0];
    puntoA.y = bezierCtrlPoints[0][0][1];
    puntoA.z = bezierCtrlPoints[0][0][2];
    puntoB.x = bezierCtrlPoints[0][1][0];
    puntoB.y = bezierCtrlPoints[0][1][1];
    puntoB.z = bezierCtrlPoints[0][1][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[0][0][0];
    puntoA.y = bezierCtrlPoints[0][0][1];
    puntoA.z = bezierCtrlPoints[0][0][2];
    puntoB.x = bezierCtrlPoints[1][0][0];
    puntoB.y = bezierCtrlPoints[1][0][1];
    puntoB.z = bezierCtrlPoints[1][0][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[0][0][0];
    puntoA.y = bezierCtrlPoints[0][0][1];
    puntoA.z = bezierCtrlPoints[0][0][2];
    puntoB.x = bezierCtrlPoints[1][1][0];
    puntoB.y = bezierCtrlPoints[1][1][1];
    puntoB.z = bezierCtrlPoints[1][1][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[0][3][0];
    puntoA.y = bezierCtrlPoints[0][3][1];
    puntoA.z = bezierCtrlPoints[0][3][2];
    puntoB.x = bezierCtrlPoints[1][3][0];
    puntoB.y = bezierCtrlPoints[1][3][1];
    puntoB.z = bezierCtrlPoints[1][3][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[0][3][0];
    puntoA.y = bezierCtrlPoints[0][3][1];
    puntoA.z = bezierCtrlPoints[0][3][2];
    puntoB.x = bezierCtrlPoints[0][2][0];
    puntoB.y = bezierCtrlPoints[0][2][1];
    puntoB.z = bezierCtrlPoints[0][2][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[0][3][0];
    puntoA.y = bezierCtrlPoints[0][3][1];
    puntoA.z = bezierCtrlPoints[0][3][2];
    puntoB.x = bezierCtrlPoints[1][2][0];
    puntoB.y = bezierCtrlPoints[1][2][1];
    puntoB.z = bezierCtrlPoints[1][2][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[3][0][0];
    puntoA.y = bezierCtrlPoints[3][0][1];
    puntoA.z = bezierCtrlPoints[3][0][2];
    puntoB.x = bezierCtrlPoints[3][1][0];
    puntoB.y = bezierCtrlPoints[3][1][1];
    puntoB.z = bezierCtrlPoints[3][1][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[3][0][0];
    puntoA.y = bezierCtrlPoints[3][0][1];
    puntoA.z = bezierCtrlPoints[3][0][2];
    puntoB.x = bezierCtrlPoints[2][0][0];
    puntoB.y = bezierCtrlPoints[2][0][1];
    puntoB.z = bezierCtrlPoints[2][0][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[3][0][0];
    puntoA.y = bezierCtrlPoints[3][0][1];
    puntoA.z = bezierCtrlPoints[3][0][2];
    puntoB.x = bezierCtrlPoints[2][1][0];
    puntoB.y = bezierCtrlPoints[2][1][1];
    puntoB.z = bezierCtrlPoints[2][1][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[3][3][0];
    puntoA.y = bezierCtrlPoints[3][3][1];
    puntoA.z = bezierCtrlPoints[3][3][2];
    puntoB.x = bezierCtrlPoints[3][2][0];
    puntoB.y = bezierCtrlPoints[3][2][1];
    puntoB.z = bezierCtrlPoints[3][2][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[3][3][0];
    puntoA.y = bezierCtrlPoints[3][3][1];
    puntoA.z = bezierCtrlPoints[3][3][2];
    puntoB.x = bezierCtrlPoints[2][3][0];
    puntoB.y = bezierCtrlPoints[2][3][1];
    puntoB.z = bezierCtrlPoints[2][3][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[3][3][0];
    puntoA.y = bezierCtrlPoints[3][3][1];
    puntoA.z = bezierCtrlPoints[3][3][2];
    puntoB.x = bezierCtrlPoints[2][2][0];
    puntoB.y = bezierCtrlPoints[2][2][1];
    puntoB.z = bezierCtrlPoints[2][2][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[1][2][0];
    puntoA.y = bezierCtrlPoints[1][2][1];
    puntoA.z = bezierCtrlPoints[1][2][2];
    puntoB.x = bezierCtrlPoints[2][2][0];
    puntoB.y = bezierCtrlPoints[2][2][1];
    puntoB.z = bezierCtrlPoints[2][2][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[1][2][0];
    puntoA.y = bezierCtrlPoints[1][2][1];
    puntoA.z = bezierCtrlPoints[1][2][2];
    puntoB.x = bezierCtrlPoints[1][1][0];
    puntoB.y = bezierCtrlPoints[1][1][1];
    puntoB.z = bezierCtrlPoints[1][1][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[2][1][0];
    puntoA.y = bezierCtrlPoints[2][1][1];
    puntoA.z = bezierCtrlPoints[2][1][2];
    puntoB.x = bezierCtrlPoints[1][1][0];
    puntoB.y = bezierCtrlPoints[1][1][1];
    puntoB.z = bezierCtrlPoints[1][1][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);
    //
    puntoA.x = bezierCtrlPoints[2][1][0];
    puntoA.y = bezierCtrlPoints[2][1][1];
    puntoA.z = bezierCtrlPoints[2][1][2];
    puntoB.x = bezierCtrlPoints[2][2][0];
    puntoB.y = bezierCtrlPoints[2][2][1];
    puntoB.z = bezierCtrlPoints[2][2][2];
    puntoA = findWarpedPoint(dst, src, puntoA);
    puntoB = findWarpedPoint(dst, src, puntoB);
    ofLine(puntoA,puntoB);

}


// Bezier markers ----------------------------------------------
void quad::drawGridMarkers()
{
    ofSetColor(0,200,220,255);
    ofSetLineWidth(1.5);

    for(int i=0; i<=gridRows; i++)
    {
        for(int j=0; j<=gridColumns; j++)
        {
            ofVec3f punto;
            punto.x = gridPoints[i][j][0]*ofGetWidth();
            punto.y = gridPoints[i][j][1]*ofGetHeight();
            punto.z = 0.0;
            punto = findWarpedPoint(dst, src, punto);
            if(bHighlightCtrlPoint && highlightedCtrlPointRow == i && highlightedCtrlPointCol == j)
            {
                ofFill();
            }
            ofCircle(punto.x, punto.y, 3.0);
            ofNoFill();
        }
    }
}

// Mask markers --------------------------------------
void quad::drawMaskMarkers()
{
    if (maskPoints.size()>0)
    {
        ofPolyline contour;
        for(unsigned int i = 0; i < maskPoints.size(); i++)
        {
            ofVec3f contourPoint;
            contourPoint.x = maskPoints[i].x;
            contourPoint.y = maskPoints[i].y;
            contourPoint.z = 0;
            contourPoint = findWarpedPoint(dst, src, contourPoint);
            contour.addVertex(contourPoint);
        }
        ofSetLineWidth(1.6);
        contour.close();
        contour.draw();

        for(unsigned int i = 0; i < maskPoints.size(); i++)
        {
            ofVec3f punto;
            punto.x = maskPoints[i].x;
            punto.y = maskPoints[i].y;
            punto.z = 0.0;
            punto = findWarpedPoint(dst, src, punto);
            ofSetColor(100,139,150,255);
            ofSetLineWidth(1.0);
            if(bHighlightMaskPoint && highlightedMaskPoint == i)
            {
                ofFill();
            }
            ofCircle(punto.x, punto.y, 4);
            ofNoFill();
            ofCircle(punto.x, punto.y, 10);
        }
    }
}
