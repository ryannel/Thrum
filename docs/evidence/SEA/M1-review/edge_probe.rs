use thrum_sea_model::{Forcing, Spectrum, Swell, LongWaves, Transition};
fn main() {
    let base = Forcing { wind_speed: 0.0, wind_direction: 0.0, fetch: 1e8, swells: vec![] };
    let flat = Spectrum::build(&base).unwrap();
    assert_eq!(flat.variance(), 0.0);
    assert_eq!(LongWaves::new(flat.long.clone(), 1.0).unwrap().evaluate([20000.0,-17000.0],86400.0).unwrap().height,0.0);
    for wavelength in [0.1, 19.99, 20.0, 20.01, 120.0, 10000.0] {
        let mut forcing = base.clone();
        forcing.swells.push(Swell { significant_height: 2.0, wavelength, direction: 2.3 });
        let sea = Spectrum::build(&forcing).unwrap();
        let height = sea.significant_height();
        assert!((height-2.0).abs() < 2e-5, "swell {wavelength}: {height}");
        println!("pure swell wavelength={wavelength} Hs={height:.12}");
    }
    let gale = Spectrum::build(&Forcing { wind_speed: 20.0, wind_direction: 2.8, ..base }).unwrap();
    let waves = LongWaves::new(gale.long.clone(),0.1).unwrap();
    let point = [20000.0,-17000.0];
    let time = 86400.0;
    let sample = waves.evaluate(point,time).unwrap();
    let eps=0.001;
    let fd=(waves.evaluate(point,time+eps).unwrap().height-waves.evaluate(point,time-eps).unwrap().height)/(2.0*eps);
    assert!((fd-sample.height_dt).abs()<1e-6);
    println!("192-component gale Hs={:.12}, far/day dt error={:.3e}",gale.significant_height(),(fd-sample.height_dt).abs());
    let transition=Transition::new(gale.clone(),flat.clone(),time,4.0).unwrap();
    assert_eq!(transition.at(time-100.0).unwrap().long,gale.long);
    for band in 0..3 {
        let before=gale.cascades[band].direct(point,time).displacement[1];
        let at=transition.at(time).unwrap().cascades[band].direct(point,time).displacement[1];
        assert_eq!(before,at);
    }
    assert!(transition.at(time+1000.0).unwrap().variance()<1e-100);
    println!("gale-to-flat: pre-start, exact boundary on all bands, asymptotic decay PASS");
}
