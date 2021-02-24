//! Provides functions to sample objects' count and size from a
//! probability distribution.
use aux::*;
use rand::Rng;
use rand_distr::Distribution;
use rand_distr;
use std::{ str, fs };

// Number of tries per sample. If no sampled number satisfies a specified
// threshold after `SAMPLE_LIMIT` tries the sampling function returns Err.
const SAMPLE_LIMIT: usize = 30;

// Probability distribution
pub struct Dist {
    pub name  : String                 ,
    pub params: Vec<f64>               , // For predefined distributions these are the params (eg mean, lambda, etc). For custom, these are the probabilities
    pub values: Option<Vec<Vec<usize>>>, // Only for custom, the values
}

/// Parses a given distribution from the config file
impl Dist {

    /// Construct a Distributions object.
    pub fn from(dist: &str) -> Result<Dist,String> {

        if dist.ends_with(".dist") {

            // A distribution file has been given
            let res = stringify_error(fs::read_to_string(dist.clone()));

            if res.is_err() {
                eprint!("libalpaca: cannot open {}: \n", dist);
            }

            let data = res?;

            // Construct the 2 vectors containing the values and probabilities
            let mut values: Vec<Vec<usize>> = Vec::new();
            let mut probs : Vec<f64>        = Vec::new();

            for line in data.lines() {

                let l            = String::from(line);
                let v: Vec<&str> = l.split_whitespace().collect();

                if values.len() > 0 && v.len() != values[0].len()+1 {
                    return Err( format!("invalid dist file {}, line {}", dist, line) );
                }

                probs .push( v[0].parse().unwrap() );
                values.push( v[1..].iter().map(|e| e.parse().unwrap()).collect() );
            }

            return Ok(Dist {
                name  : String::from("custom"),
                params: probs                 ,
                values: Some(values)          ,
            });

        } else if dist == "" || dist == "Joint" {

            return Ok( Dist {
                name  : String::from(dist),
                params: Vec::new()        ,
                values: None              ,
            });

        } else {

            let tokens: Vec<&str> = dist.split("/").collect();

            if tokens.len() != 2 {
                return Err( format!("invalid distribution {}", dist) );
            }

            let name             = tokens[0];
            let params: Vec<f64> = tokens[1].split(",").map( |s| s.parse().unwrap() ).collect(); // Distributions parameters

            let params_needed = match name {
                "Normal"    => 2,
                "LogNormal" => 2,
                "Exp"       => 1,
                "Poisson"   => 1,
                "Binomial"  => 2,
                "Gamma"     => 2,
                _           => return Err( format!("invalid distribution {}", dist) ),
            };

            // A predefined distribution and its parameters have been given.
            if params.len() != params_needed {
                return Err( format!( "{} distribution requires {} params, {} given", name, params_needed, params.len() ) );
            }

            return Ok(Dist {
                name  : String::from(name),
                params: params            ,
                values: None              ,
            });
        }
    }
}

pub fn sample_ge_many(dist:&Dist, lower_bound:usize, samples:usize) -> Result<Vec<usize>,String> {

    let mut vec: Vec<usize> = Vec::new();

    for _ in 0..samples {
        vec.push( sample_ge(dist, lower_bound)? );
    }

    Ok(vec)
}

/// Samples a value greater or equal than the given one
pub fn sample_ge(dist:&Dist, lower_bound:usize) -> Result<usize,String> {

    if dist.name == "custom" {

        let values = dist.values.as_ref().unwrap();

        if values[0].len() != 1 {
            return Err( format!( "alpaca: custom distribution contains {} values per row, expected 1", values[0].len() ) );
        }

        // Sample from custom distribution in a single try, by considering only values >= lower_bound
        let total_mass:f64 = ( 0..values.len() ).filter( |i| values[*i][0] >= lower_bound ).map( |i| dist.params[i] ).sum();

        if total_mass < 1e-5 {
            return Err( format!("values >= {} have prob 0 in custom distribution", lower_bound) );
        }

        let probability: f64 = rand::thread_rng().sample(rand_distr::OpenClosed01);
        let mut sum          = 0.0;
        let mut sampled_num  = 0;

        // Sample a value from the given distribution
        for i in 0..values.len() {
            if values[i][0] >= lower_bound {

                sampled_num  = values[i][0];            // make sure we keep one
                sum         += dist.params[i] / total_mass;

                if sum >= probability {
                    break;
                }
            }
        }

        Ok(sampled_num)

    } else if dist.name == "" {
        // Empty dist means use the real value
        Ok(lower_bound)

    } else {

        for _ in 0..SAMPLE_LIMIT {

            let sampled_num = sample_predefined(dist);

            if sampled_num >= lower_bound {
                return Ok(sampled_num);
            }
        }

        Err( format!("SAMPLE_LIMIT={} reached for distribution {}", SAMPLE_LIMIT, dist.name) )
    }
}

// returns a pair (a,b) from a joint distribution, satisfying
//    a >= lb_a   and   b >= lb_b      where (a,b) = lower_bound
//
pub fn sample_pair_ge( dist: &Dist, lower_bound: (usize, usize) ) -> Result<(usize,usize), String> {

    if dist.name != "custom" {
        return Err( format!( "alpaca: joint distributions need to be given in a file (got: {})", dist.name) );
    }

    let values = dist.values.as_ref().unwrap();

    if values[0].len() != 2 {
        return Err( format!( "alpaca: custom distribution contains {} values per row, expected 2", values[0].len() ) );
    }

    // Sample from custom distribution in a single try, by considering only values >= lower_bound
    let (lb_a, lb_b) = lower_bound;

    let total_mass: f64 = ( 0..values.len() ).filter( |i| values[*i][0] >= lb_a && values[*i][1] >= lb_b ).map( |i| dist.params[i] ).sum();

    if total_mass < 1e-5 {
        return Err( format!("values >= ({},{}) have prob 0 in custom distribution", lb_a, lb_b) );
    }

    let probability: f64 = rand::thread_rng().sample( rand_distr::OpenClosed01 );

    let mut sum       = 0.0;
    let mut sampled_a = 0;
    let mut sampled_b = 0;

    // Sample a value from the given distribution
    for i in 0..values.len() {

        if values[i][0] >= lb_a && values[i][1] >= lb_b {

            sampled_a  = values[i][0];            // make sure
            sampled_b  = values[i][1];            // we keep one
            sum       += dist.params[i] / total_mass;

            if sum >= probability {
                break;
            }
        }
    }

    Ok( (sampled_a, sampled_b) )
}

fn sample_predefined(dist: &Dist) -> usize {

    match dist.name.as_str() {

        "Normal" => {
            let d = rand_distr::Normal::new( dist.params[0], dist.params[1] ).unwrap();
            d.sample( &mut rand::thread_rng() ) as usize
        },

        "LogNormal" => {
            let d = rand_distr::LogNormal::new( dist.params[0], dist.params[1] ).unwrap();
            d.sample( &mut rand::thread_rng() ) as usize
        },

        "Exp" => {
            let d = rand_distr::Exp::new( dist.params[0] ).unwrap();
            d.sample( &mut rand::thread_rng() ) as usize
        },

        // "Poisson" => {
        //     let d = Poisson::new(dist.params[0]).unwrap();
        //     return Ok(d.sample(&mut rand::thread_rng()) as usize);
        // },
        "Binomial" => {
            let d = rand_distr::Binomial::new( dist.params[0] as u64, dist.params[1] ).unwrap();
            d.sample( &mut rand::thread_rng() ) as usize
        },

        "Gamma" => {
            let d = rand_distr::Gamma::new( dist.params[0], dist.params[1] ).unwrap();
            d.sample( &mut rand::thread_rng() ) as usize
        },

        _ => panic!("not possible"),
    }
}