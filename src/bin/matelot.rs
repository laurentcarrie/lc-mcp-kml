use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Job {
    Cuisinier,
    Infirmier,
    Radio,
    Mecanicien,
}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Job::Cuisinier => write!(f, "Cuisinier"),
            Job::Infirmier => write!(f, "Infirmier"),
            Job::Radio => write!(f, "Radio"),
            Job::Mecanicien => write!(f, "Mécanicien"),
        }
    }
}

const JOBS: [Job; 4] = [Job::Cuisinier, Job::Infirmier, Job::Radio, Job::Mecanicien];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Name {
    Boris,
    Carl,
    David,
    Alex,
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Name::Boris => write!(f, "Boris"),
            Name::Carl => write!(f, "Carl"),
            Name::David => write!(f, "David"),
            Name::Alex => write!(f, "Alex"),
        }
    }
}

const NAMES: [Name; 4] = [Name::Boris, Name::Carl, Name::David, Name::Alex];

fn main() {
    // Each assignment is [Boris's job, Carl's job, David's job, Alex's job]
    // Try all permutations of jobs assigned to people
    for b in JOBS {
        for c in JOBS {
            if c == b { continue; }
            for d in JOBS {
                if d == b || d == c { continue; }
                for a in JOBS {
                    if a == b || a == c || a == d { continue; }

                    let assignment = [(Name::Boris, b), (Name::Carl, c), (Name::David, d), (Name::Alex, a)];

                    // A1: Alex is Infirmier AND Boris is Cuisinier
                    let a1 = a == Job::Infirmier && b == Job::Cuisinier;
                    // A2: Boris is Cuisinier AND Carl is Radio
                    let a2 = b == Job::Cuisinier && c == Job::Radio;
                    // A3: Carl is Radio AND David is Mecanicien
                    let a3 = c == Job::Radio && d == Job::Mecanicien;
                    // A4: David is Infirmier AND Alex is Cuisinier
                    let a4 = d == Job::Infirmier && a == Job::Cuisinier;

                    // Carl is Radio
                    if c != Job::Radio { continue; }

                    // Exactly one assertion is true
                    let true_count = [a1, a2, a3, a4].iter().filter(|&&x| x).count();
                    if true_count == 1 {
                        let which = if a1 { "A1" } else if a2 { "A2" } else if a3 { "A3" } else { "A4" };
                        println!("Solution (assertion {which} is true):");
                        for (name, job) in &assignment {
                            println!("  {name} -> {job}");
                        }
                        println!();
                    }
                }
            }
        }
    }
}
