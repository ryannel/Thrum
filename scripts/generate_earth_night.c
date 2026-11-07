// Regenerate the fixed Earth night transport profile.
// cc -std=c17 -O3 scripts/generate_earth_night.c -lm -o /tmp/thrum-earth-night
// /tmp/thrum-earth-night crates/thrum-render/src/atmosphere/night/earth.rg16f
//
// Thrum's spherical first-scatter model. See docs/design/modules/ATMOSPHERE.md.
// The stored RG16Float channels are terminal luminance and local source density.
#include <math.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>

static const double R = 6371000, INNER = 85000, SHELL = 95000, PI = 3.141592653589793;
static const double B_R = 1.3536325e-5, B_M = 4.4e-6, S_M = 3.996e-6, B_O = 1.885528e-6;

static double horizon(double h) {
    return -sqrt(h * (2 * R + h)) / (R + h);
}

static double height_at(double h, double mu, double s) {
    const double r = R + h;
    const double d2 = (s + 2 * r * mu) * s;
    return fmax(0, h + d2 / (sqrt(r*r + d2) + r));
}

static double path(double h, double mu, double end_h) {
    const double r = R + h, b = r * mu;
    const double c = (end_h - h) * (2 * R + end_h + h);
    const double q = sqrt(fmax(0, b*b + c));
    if (end_h == 0) return -c / fmax(1e-12, -b + q);
    if (mu >= 0) return c / fmax(1e-12, q + b);
    return q - b;
}

static double extinction(double h) {
    return B_R*exp(-h/8000) + B_M*exp(-h/1200) + B_O*fmax(0, 1 - fabs(h-25000)/15000);
}

static double transmission(double h, double mu, double distance) {
    double tau = 0;
    const unsigned steps = 80;
    for (unsigned k = 0; k < steps; k++) {
        const double f = (double)k / steps, g = (double)(k+1) / steps;
        const double a = f*f*distance, b = g*g*distance;
        tau += extinction(height_at(h,mu,0.5*(a+b))) * (b-a);
    }
    return exp(-tau);
}

static double shell(double h, double mu) {
    const double end = path(h,mu,SHELL);
    const double r = R+h;
    const double b = r*mu;
    const double d = b*b+(INNER-h)*(2*R+INNER+h);
    if (d <= 0) return end/(SHELL-INNER);
    const double q = sqrt(d);
    const double interior = fmax(0, fmin(end,-b+q)-fmax(0,-b-q));
    return fmax(0,end-interior)/(SHELL-INNER);
}

static double source(double h, double view_mu, bool diffuse) {
    const double lower = horizon(h);
    const unsigned strips = 40, sectors = 48;
    const double r = B_R*exp(-h/8000), m = S_M*exp(-h/1200);
    const double a = sqrt(fmax(0,1-view_mu*view_mu));
    const double g = 0.8;
    double result = 0;
    for (unsigned i = 0; i < strips; i++) {
        // Cluster incoming angles near the tangent, where transmission changes quickly.
        const double u = (double)i/strips, v = (double)(i+1)/strips;
        const double start = lower+(1-lower)*u*u, end = lower+(1-lower)*v*v;
        const double mu = 0.5*(start+end), radial = sqrt(fmax(0,1-mu*mu));
        const double incoming = diffuse ? 1.0 : shell(h,mu);
        const double sky = incoming * transmission(h,mu,path(h,mu,SHELL));
        double scatter = 0;
        for (unsigned j = 0; j < sectors; j++) {
            const double phi = 2*PI*(j+0.5)/sectors;
            const double c = mu*view_mu + a*radial*cos(phi);
            const double q = 1+g*g-2*g*c;
            const double rayleigh = 3*(1+c*c)/(16*PI);
            const double mie = (1-g*g)/(4*PI*q*sqrt(q));
            scatter += r*rayleigh + m*mie;
        }
        result += sky * scatter * (end-start) * 2*PI/sectors;
    }
    return result;
}

static double trace_sky(double h, double mu, bool diffuse) {
    const double r = R+h, distance = path(h,mu,SHELL);
    double trans = 1, integral = 0;
    const unsigned steps = 64;
    for (unsigned k = 0; k < steps; k++) {
        const double f = (double)k/steps, g = (double)(k+1)/steps;
        const double a = f*f*distance, b = g*g*distance, s = 0.5*(a+b);
        const double ah = height_at(h,mu,s);
        const double local_mu = (s+r*mu)/(R+ah);
        const double ext = extinction(ah);
        const double segment = exp(-ext*(b-a));
        integral += trans * source(ah,local_mu,diffuse) * (-expm1(-ext*(b-a)))/ext;
        trans *= segment;
    }
    const double direct = trans * (diffuse ? 1.0 : shell(h,mu));
    return direct+integral;
}


static double horizontal(double height, bool diffuse) {
    const unsigned count = 64;
    double sum = 0;
    for (unsigned i = 0; i < count; i++) {
        const double mu = (i+0.5)/count;
        sum += trace_sky(height,mu,diffuse)*mu*2*PI/count;
    }
    return sum;
}

enum { HEIGHTS = 64, WIDTH = 96, HALF_WIDTH = WIDTH/2 };
static const double OBSERVER_TOP = 15000.0;
static const double TERMINAL_GAIN = 65536.0;
static const double SOURCE_GAIN = 1073741824.0;
static _Float16 values[HEIGHTS][WIDTH][2];

static double lerp(double a, double b, double fraction) {
    return a+(b-a)*fraction;
}

static double signed_square(double x) {
    return copysign(x*x,x);
}

static double local_mu(unsigned index) {
    const double coordinate = 2.0*index/(WIDTH-1)-1.0;
    return signed_square(coordinate);
}

static double terminal_mu(double height, unsigned index, bool ground) {
    const double tangent = horizon(height);
    const double p = (double)index/(HALF_WIDTH-1);
    return ground?lerp(tangent,-1,p*p):lerp(tangent,1,p*p);
}

static double texture_read(double h_coordinate, double x_coordinate, unsigned first, unsigned last, unsigned channel) {
    const double y = fmin(HEIGHTS-1,fmax(0,h_coordinate));
    const double x = fmin(last,fmax(first,x_coordinate));
    const unsigned j=(unsigned)y, i=(unsigned)x;
    const unsigned j1=j+1<HEIGHTS?j+1:j, i1=i<last?i+1:i;
    const double a=lerp(values[j][i][channel],values[j][i1][channel],x-i);
    const double b=lerp(values[j1][i][channel],values[j1][i1][channel],x-i);
    return lerp(a,b,y-j);
}

static double lookup_source(double h, double mu) {
    return texture_read(sqrt(h/SHELL)*(HEIGHTS-1),
                        (copysign(sqrt(fabs(mu)),mu)+1)*0.5*(WIDTH-1),0,WIDTH-1,1)/SOURCE_GAIN;
}

static double local_segment(double h, double mu, bool ground, double ground_l, double terminal) {
    const double r = R+h, distance = path(h,mu,ground?0:SHELL);
    double trans=1, integral=0;
    for (unsigned k=0; k<96; k++) {
        const double f=(double)k/96, g=(double)(k+1)/96;
        const double a=f*f*distance, b=g*g*distance, mid=0.5*(a+b);
        const double ah=height_at(h,mu,mid), m=(mid+r*mu)/(R+ah);
        const double ext=extinction(ah), segment=exp(-ext*(b-a));
        integral+=trans*lookup_source(ah,m)*(-expm1(-ext*(b-a)))/ext;
        trans*=segment;
    }
    return integral+trans*(ground?ground_l:terminal);
}

int main(int argc, char **argv) {
    if (argc != 2) return 1;
    double weights[2], ground_e=0;
    for (unsigned component=0; component<2; component++) {
        const bool diffuse=component==1;
        weights[component]=(diffuse?0.4:0.6)*0.000171/trace_sky(0,1,diffuse);
        ground_e+=horizontal(0,diffuse)*weights[component];
    }
    for (unsigned j=0; j<HEIGHTS; j++) {
        // Cell just inside the atmospheric cap. The finite shell stays
        // bounded, including an observer local to the shell.
        const double q=(double)j/(HEIGHTS-1), h=fmin(SHELL-0.1,q*q*SHELL);
        for (unsigned i=0; i<WIDTH; i++) {
            const double m=local_mu(i);
            const double beam=source(h,m,false)*weights[0];
            const double residual=source(h,m,true)*weights[1];
            values[j][i][1]=(_Float16)((beam+residual)*SOURCE_GAIN);
        }
    }
    for (unsigned j=0; j<HEIGHTS; j++) {
        const double q=(double)j/(HEIGHTS-1),h=q*q*OBSERVER_TOP;
        for (unsigned i=0; i<WIDTH; i++) {
            const bool ground=i<HALF_WIDTH;
            const double m=terminal_mu(h,i%HALF_WIDTH,ground);
            const double terminal=shell(h,m)*weights[0]+weights[1];
            const double result=local_segment(h,m,ground,ground_e*0.1/PI,terminal);
            values[j][i][0]=(_Float16)(result*TERMINAL_GAIN);
        }
    }
    FILE *data=fopen(argv[1],"wb");
    if (!data) return 2;
    if (fwrite(values,sizeof values,1,data)!=1 || fclose(data)!=0) return 3;
    return 0;
}
