# Facial rendering source scan: P1nkL1on/SkinShader

Repository: https://github.com/P1nkL1on/SkinShader

Pinned source revision: b7bd2f73a134ee18982044b13ac558abec906baa

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"sss": ["TestShader/Shading/crossshading.cpp", "TestShader/Shading/crossshading.h"], "shader": ["TestShader/Shading/Shading.pri", "TestShader/Shading/ShadingTests.pri", "TestShader/Shading/crossshading.cpp", "TestShader/Shading/crossshading.h", "TestShader/Shading/planevector.cpp", "TestShader/Shading/planevector.h", "TestShader/Shading/planevectordrawer.cpp", "TestShader/Shading/planevectordrawer.h", "TestShader/Shading/planevectortests.cpp", "TestShader/Shading/planevectortests.h", "TestShader/Shading/point.cpp", "TestShader/Shading/point.h", "TestShader/Shading/trianglespeller.cpp", "TestShader/Shading/trianglespeller.h", "TestShader/TestShader.pro", "TestShader/TestShader.pro.user", "TestShader/main.cpp", "TestShader/mainwindow.cpp", "TestShader/mainwindow.h", "TestShader/mainwindow.ui"]}

## Source file: TestShader/Shading/crossshading.h

{
  "path": "TestShader/Shading/crossshading.h",
  "blob_sha": "87d699a4f8453fc87a00c57a91c0ede0fd33a1b8",
  "sha256": "b0b602f97bfb1d3ea80e1356d6b7f19ee9ecb8782daf1be94608435eeebc9e74",
  "lines": 53,
  "bytes": 2068,
  "selection_score": 16,
  "keyword_line_counts": {
    "sss": 5,
    "rig": 5
  },
  "url": "https://github.com/P1nkL1on/SkinShader/blob/b7bd2f73a134ee18982044b13ac558abec906baa/TestShader/Shading/crossshading.h",
  "function_signature_candidates": [
    {
      "line": 11,
      "text": "void smooth (const PlaneVector &v, PlaneVector &to, const int interpolation) const;"
    },
    {
      "line": 12,
      "text": "void blurInDirection ("
    },
    {
      "line": 17,
      "text": "void blurCross("
    },
    {
      "line": 29,
      "text": "void blurPixelsInTriangleInDirect("
    },
    {
      "line": 38,
      "text": "void blurPixelsInTriangleCross("
    }
  ]
}

```text

#ifndef CROSSSHADING_H
#define CROSSSHADING_H

#include "planevector.h"


class CrossShading
{
public:
    CrossShading();
    void smooth (const PlaneVector &v, PlaneVector &to, const int interpolation) const;
    void blurInDirection (
            const PlaneVector &originalImage,
            PlaneVector &resultImage,
            const double r, const double directionX, const double directionY,
            const int interpolationType) const;
    void blurCross(
            const PlaneVector &originalImage,
            PlaneVector &resultImage,
            const double rs, const double rt, const double directX, const double directY,
            const int interpolationType) const;

    double blurPixelInDirection(
            const PlaneVector &originalImage,
            const int x, const int y,
            const double r, const double directionX, const double directionY,
            const int interpolationType) const;

    void blurPixelsInTriangleInDirect(
            const PlaneVector &originalImage,
            PlaneVector &resImage,
            const float x0, const float y0,
            const float x1, const float y1,
            const float x2, const float y2,
            const double r,
            const double directionX, const double directionY,
            const int interpolationType) const;
    void blurPixelsInTriangleCross(
            const PlaneVector &originalImage,
            PlaneVector &resImage,
            const float x0, const float y0,
            const float x1, const float y1,
            const float x2, const float y2,
            const double rs, const double rt,
            const double directionX, const double directionY,
            const int interpolationType) const;
private:
    double getC(const int center, const double tan, const int number, const int max) const;
    double getInVec(const PlaneVector &v, const int x, const int y, const double xvec, const double yvec1, const double K, const int interPolation) const;
    double getGaussian(double delt, double x) const;
};

#endif // CROSSSHADING_H


```

## Source file: TestShader/Shading/crossshading.cpp

{
  "path": "TestShader/Shading/crossshading.cpp",
  "blob_sha": "927bdc32a7cb7914e1ceae5287f92cda4e609376",
  "sha256": "6231c353f7bf09c2bc25bbd1c38dfea8a9746b05d2f084060269ffe67301a72b",
  "lines": 188,
  "bytes": 7767,
  "selection_score": 16,
  "keyword_line_counts": {
    "sss": 11,
    "rig": 18
  },
  "url": "https://github.com/P1nkL1on/SkinShader/blob/b7bd2f73a134ee18982044b13ac558abec906baa/TestShader/Shading/crossshading.cpp",
  "function_signature_candidates": []
}

```text

#include "crossshading.h"
#include "math.h"
#include "qdebug.h"
CrossShading::CrossShading()
{

}

void CrossShading::smooth(const PlaneVector &v, PlaneVector &to, const int interpolation) const
{
    const double r = 1;
//    double sigma = (r>=1.0)? (38.2 * r - 26.5) : (70.5 * r - 46.9);

    const int rad = 31;
    const double numSigma = 2.5;

    const double xvec = 1.0, yvec = 1.0;
    double L = sqrt(xvec * xvec + yvec * yvec);
    double xvec1 = xvec / L;
    double yvec1 = yvec / L;


    for (int x = 0; x < to.width(); ++x)
        for (int y = 0; y < to.height(); ++y){

            double summ = 0;
            double summWeigth = 0;
            for (int i = 0; i <= rad; ++i){
                const double val = ( i - rad / 2.0) * numSigma / (rad + 1);
                const double weigth = getGaussian(1, val);
                summWeigth += weigth;
                summ += weigth * getInVec(v, x, y, xvec1, yvec1, i - rad / 2.0, interpolation);
            }
            to.setValue(summ / summWeigth, x, y);
        }
}

void CrossShading::blurInDirection(
        const PlaneVector &originalImage, PlaneVector &resultImage,
        const double s, const double directionX, const double directionY,
        const int interpolationType) const
{
    const double r = std::min(1.4, std::max(.7, s));

    const double sigma = (r >= 1.0)? (38.2 * r - 26.5) : (70.5 * r - 46.9);
    const double alpha = std::min(1.0, std::min(15.4*r-13.8, 3.09*(r-1.0)));

    const int rad = 31;
    const double numSigma = 2.5;

    double directionLength = sqrt(directionX * directionX + directionY * directionY);
    double xComponent = directionX / directionLength;
    double yComponent = directionY / directionLength;

    for (int x = 0; x < originalImage.width(); ++x)
        for (int y = 0; y < originalImage.height(); ++y){
            double summWeigth = 0;
            double gausWs[rad];
            for (int i = 0; i <= rad; ++i){
                const double val = ( i - rad / 2.0) * numSigma / (rad - 1.0);
                gausWs[i] = getGaussian(1, val);
                summWeigth += gausWs[i];
            }
            double resVal = 0.0;
            for (int i = 0; i <= rad; ++i){
                const double sx = ( i - rad / 2.0) * numSigma / (rad + 1) * sigma;
                const double colorK = (((i == rad/2)? (1.0 - alpha) : (0.0)) + alpha * gausWs[i] / summWeigth);
                resVal += getInVec(originalImage, x, y, xComponent, yComponent, sx, interpolationType) * colorK;
            }
            resultImage.setValue(resVal, x, y);
        }
}

void CrossShading::blurCross(const PlaneVector &originalImage, PlaneVector &resultImage,
                             const double rs, const double rt, const double directX, const double directY,
                             const int interpolationType) const
{
    const double boost = .1;
    PlaneVector timed = PlaneVector(originalImage.width(), originalImage.height());
    blurInDirection(originalImage, timed, rs + boost, directX, directY, interpolationType);
    blurInDirection(timed, resultImage, rt - boost, directY, directX, interpolationType);

//    blurInDirection(originalImage, resultImage, rs + boost, directX, directY, interpolationType);
}

double CrossShading::blurPixelInDirection(const PlaneVector &originalImage,
                                          const int x, const int y, const double s,
                                          const double directionX, const double directionY,
                                          const int interpolationType) const
{
    const double r = std::min(1.4, std::max(.7, s));

    const double sigma = (r >= 1.0)? (38.2 * r - 26.5) : (70.5 * r - 46.9);
    const double alpha = std::min(1.0, std::min(15.4*r-13.8, 3.09*(r-1.0)));

    const int rad = 31;
    const double numSigma = 2.5;

    double directionLength = sqrt(directionX * directionX + directionY * directionY);
    double xComponent = directionX / directionLength;
    double yComponent = directionY / directionLength;

    double summWeigth = 0;
    double gausWs[rad];
    for (int i = 0; i <= rad; ++i){
        const double val = ( i - rad / 2.0) * numSigma / (rad - 1.0);
        gausWs[i] = getGaussian(1, val);
        summWeigth += gausWs[i];
    }
    double resVal = 0.0;
    for (int i = 0; i <= rad; ++i){
        const double sx = ( i - rad / 2.0) * numSigma / (rad + 1) * sigma;
        const double colorK = (((i == rad/2)? (1.0 - alpha) : (0.0)) + alpha * gausWs[i] / summWeigth);
        resVal += getInVec(originalImage, x, y, xComponent, yComponent, sx, interpolationType) * colorK;
    }
    return resVal;
}

void CrossShading::blurPixelsInTriangleInDirect(
    const PlaneVector &originalImage, PlaneVector &resImage,
    const float x0, const float y0, const float x1, const float y1,
    const float x2, const float y2, const double r,
    const double directionX, const double directionY, const int interpolationType) const
{
    int
        X0 = int(x0 * originalImage.width()), Y0 = int(y0 * originalImage.height()),
        X1 = int(x1 * originalImage.width()), Y1 = int(y1 * originalImage.height()),
        X2 = int(x2 * originalImage.width()), Y2 = int(y2 * originalImage.height());

    if (Y0 == Y1 && Y0 == Y2) return;
    if (Y0 > Y1){ std::swap(Y0, Y1); std::swap(X0, X1); }
    if (Y0 > Y2){ std::swap(Y0, Y2); std::swap(X0, X2); }
    if (Y1 > Y2){ std::swap(Y1, Y2); std::swap(X1, X2); }

    int total_height = Y2 - Y0;
    for (int i = 0; i < total_height; ++i){
        bool second_half = (i > Y1 - Y0) || (Y1 == Y0);
        int segment_height = second_half? Y2 - Y1 : Y1 - Y0;
        float alpha = float(i) / total_height;
        float beta = float(i-(second_half? (Y1 - Y0) : 0)) / segment_height;
        float XA = X0 + (X2 - X0)*alpha;
        float XB = second_half? (X1 + (X2 - X1) * beta) : (X0 + (X1 - X0) * beta);
        if (XA > XB) std::swap(XA, XB);
        for (int j = int(XA); j < int(XB); ++j ){
            //ballicentrate(j, Y0 + i, X0, Y0, X1, Y1, X2, Y2, a,b,c);
            //v.setValue(a * .0 + b * 50.0 + c * 100.0, j, Y0 + i);
            resImage.setValue(
                blurPixelInDirection(
                    originalImage, j, Y0 + i,
                    r, directionX, directionY,
                    interpolationType),
                j, Y0 + i);
        }
    }
}

void CrossShading::blurPixelsInTriangleCross(const PlaneVector &originalImage, PlaneVector &resImage, const float x0, const float y0, const float x1, const float y1, const float x2, const float y2, const double rs, const double rt, const double directionX, const double directionY, const int interpolationType) const
{
    const double boost = .1;
    PlaneVector timed = PlaneVector(originalImage.width(), originalImage.height());
    blurPixelsInTriangleInDirect(originalImage, timed, x0, y0, x1, y1, x2, y2, rs + boost, directionX, directionY, interpolationType);
    blurPixelsInTriangleInDirect(timed, resImage, x0, y0, x1, y1, x2, y2, rt - boost, directionY, directionX, interpolationType);
}


double CrossShading::getC(const int center, const double tan, const int number, const int max) const
{
    double res = center + (tan * number);
    if (res < 0.0) return 0.0;
    if (res > max) return max * 1.0;
    return res;
}



double CrossShading::getInVec(const PlaneVector &v, const int x, const int y, const double xvec1, const double yvec1, const double K, const int interPolation) const
{
    const double getX = getC(x, xvec1, K, v.width() - 1);
    const double getY = getC(y, yvec1, K, v.height() - 1);
    return v.getValue(getX, getY, interPolation);
}

double CrossShading::getGaussian(double sigma, double x) const
{
    double xs = x / sigma;
    return exp(-.5 * xs * xs);
}



```

## Source file: TestShader/main.cpp

{
  "path": "TestShader/main.cpp",
  "blob_sha": "e36d45d77bf978119487440d0face91d74abad6e",
  "sha256": "ccbd45b48790f96928d15fd59a971a68019881feee1ccd66acc29e2ec0efcc7c",
  "lines": 11,
  "bytes": 181,
  "selection_score": 1,
  "keyword_line_counts": {},
  "url": "https://github.com/P1nkL1on/SkinShader/blob/b7bd2f73a134ee18982044b13ac558abec906baa/TestShader/main.cpp",
  "function_signature_candidates": [
    {
      "line": 4,
      "text": "int main(int argc, char *argv[])"
    }
  ]
}

```text

#include "mainwindow.h"
#include <QApplication>

int main(int argc, char *argv[])
{
    QApplication a(argc, argv);
    MainWindow w;
    w.showMaximized();

    return a.exec();
}


```

## Source file: TestShader/mainwindow.h

{
  "path": "TestShader/mainwindow.h",
  "blob_sha": "5e693da7cf1d27ca46cb819f86a15fc3d1dd5037",
  "sha256": "a4640ed5dd5edba6ad713a1d7d51a3c2483dbb6775dcf392011a5bbde2a84205",
  "lines": 37,
  "bytes": 776,
  "selection_score": 1,
  "keyword_line_counts": {
    "sss": 2
  },
  "url": "https://github.com/P1nkL1on/SkinShader/blob/b7bd2f73a134ee18982044b13ac558abec906baa/TestShader/mainwindow.h",
  "function_signature_candidates": [
    {
      "line": 24,
      "text": "void runTests() const;"
    },
    {
      "line": 32,
      "text": "void paintEvent(QPaintEvent *e);"
    },
    {
      "line": 34,
      "text": "void on_pushButton_clicked();"
    }
  ]
}

```text

#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include <QMainWindow>
#include "Shading/planevectortests.h"
#include "Shading/trianglespeller.h"
#include "Shading/planevectordrawer.h"
#include "Shading/crossshading.h"

namespace Ui {
class MainWindow;
}

class MainWindow : public QMainWindow
{
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = 0);
    ~MainWindow();

private:
    Ui::MainWindow *ui;
    void runTests() const;

    PlaneVectorDrawer *p = new PlaneVectorDrawer();
    const int imageSize = 250;
    bool everSaved = false;
    //PlaneVector::testPlaneVector(imageSize, imageSize);
    CrossShading *cs = new CrossShading();
protected:
    void paintEvent(QPaintEvent *e);
private slots:
    void on_pushButton_clicked();
};

#endif // MAINWINDOW_H


```
