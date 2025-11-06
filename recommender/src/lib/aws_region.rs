use std::fmt;
use std::str::FromStr;

/// AWS Regions as documented in https://docs.aws.amazon.com/general/latest/gr/rande.html
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AwsRegion {
    // US Regions
    UsEast1,      // US East (N. Virginia)
    UsEast2,      // US East (Ohio)
    UsWest1,      // US West (N. California)
    UsWest2,      // US West (Oregon)
    
    // Africa
    AfSouth1,     // Africa (Cape Town)
    
    // Asia Pacific
    ApEast1,      // Asia Pacific (Hong Kong)
    ApEast2,      // Asia Pacific (Taipei)
    ApSouth1,     // Asia Pacific (Mumbai)
    ApSouth2,     // Asia Pacific (Hyderabad)
    ApNortheast1, // Asia Pacific (Tokyo)
    ApNortheast2, // Asia Pacific (Seoul)
    ApNortheast3, // Asia Pacific (Osaka)
    ApSoutheast1, // Asia Pacific (Singapore)
    ApSoutheast2, // Asia Pacific (Sydney)
    ApSoutheast3, // Asia Pacific (Jakarta)
    ApSoutheast4, // Asia Pacific (Melbourne)
    ApSoutheast5, // Asia Pacific (Malaysia)
    ApSoutheast6, // Asia Pacific (New Zealand)
    ApSoutheast7, // Asia Pacific (Thailand)
    
    // Canada
    CaCentral1,   // Canada (Central)
    CaWest1,      // Canada West (Calgary)
    
    // Europe
    EuCentral1,   // Europe (Frankfurt)
    EuCentral2,   // Europe (Zurich)
    EuWest1,      // Europe (Ireland)
    EuWest2,      // Europe (London)
    EuWest3,      // Europe (Paris)
    EuNorth1,     // Europe (Stockholm)
    EuSouth1,     // Europe (Milan)
    EuSouth2,     // Europe (Spain)
    
    // Israel
    IlCentral1,   // Israel (Tel Aviv)
    
    // Mexico
    MxCentral1,   // Mexico (Central)
    
    // Middle East
    MeSouth1,     // Middle East (Bahrain)
    MeCentral1,   // Middle East (UAE)
    
    // South America
    SaEast1,      // South America (São Paulo)
    
    // AWS GovCloud
    UsGovEast1,   // AWS GovCloud (US-East)
    UsGovWest1,   // AWS GovCloud (US-West)
}

impl AwsRegion {
    pub fn as_str(&self) -> &str {
        match self {
            AwsRegion::UsEast1 => "us-east-1",
            AwsRegion::UsEast2 => "us-east-2",
            AwsRegion::UsWest1 => "us-west-1",
            AwsRegion::UsWest2 => "us-west-2",
            AwsRegion::AfSouth1 => "af-south-1",
            AwsRegion::ApEast1 => "ap-east-1",
            AwsRegion::ApEast2 => "ap-east-2",
            AwsRegion::ApSouth1 => "ap-south-1",
            AwsRegion::ApSouth2 => "ap-south-2",
            AwsRegion::ApNortheast1 => "ap-northeast-1",
            AwsRegion::ApNortheast2 => "ap-northeast-2",
            AwsRegion::ApNortheast3 => "ap-northeast-3",
            AwsRegion::ApSoutheast1 => "ap-southeast-1",
            AwsRegion::ApSoutheast2 => "ap-southeast-2",
            AwsRegion::ApSoutheast3 => "ap-southeast-3",
            AwsRegion::ApSoutheast4 => "ap-southeast-4",
            AwsRegion::ApSoutheast5 => "ap-southeast-5",
            AwsRegion::ApSoutheast6 => "ap-southeast-6",
            AwsRegion::ApSoutheast7 => "ap-southeast-7",
            AwsRegion::CaCentral1 => "ca-central-1",
            AwsRegion::CaWest1 => "ca-west-1",
            AwsRegion::EuCentral1 => "eu-central-1",
            AwsRegion::EuCentral2 => "eu-central-2",
            AwsRegion::EuWest1 => "eu-west-1",
            AwsRegion::EuWest2 => "eu-west-2",
            AwsRegion::EuWest3 => "eu-west-3",
            AwsRegion::EuNorth1 => "eu-north-1",
            AwsRegion::EuSouth1 => "eu-south-1",
            AwsRegion::EuSouth2 => "eu-south-2",
            AwsRegion::IlCentral1 => "il-central-1",
            AwsRegion::MxCentral1 => "mx-central-1",
            AwsRegion::MeSouth1 => "me-south-1",
            AwsRegion::MeCentral1 => "me-central-1",
            AwsRegion::SaEast1 => "sa-east-1",
            AwsRegion::UsGovEast1 => "us-gov-east-1",
            AwsRegion::UsGovWest1 => "us-gov-west-1",
        }
    }
}

impl fmt::Display for AwsRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for AwsRegion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "us-east-1" => Ok(AwsRegion::UsEast1),
            "us-east-2" => Ok(AwsRegion::UsEast2),
            "us-west-1" => Ok(AwsRegion::UsWest1),
            "us-west-2" => Ok(AwsRegion::UsWest2),
            "af-south-1" => Ok(AwsRegion::AfSouth1),
            "ap-east-1" => Ok(AwsRegion::ApEast1),
            "ap-east-2" => Ok(AwsRegion::ApEast2),
            "ap-south-1" => Ok(AwsRegion::ApSouth1),
            "ap-south-2" => Ok(AwsRegion::ApSouth2),
            "ap-northeast-1" => Ok(AwsRegion::ApNortheast1),
            "ap-northeast-2" => Ok(AwsRegion::ApNortheast2),
            "ap-northeast-3" => Ok(AwsRegion::ApNortheast3),
            "ap-southeast-1" => Ok(AwsRegion::ApSoutheast1),
            "ap-southeast-2" => Ok(AwsRegion::ApSoutheast2),
            "ap-southeast-3" => Ok(AwsRegion::ApSoutheast3),
            "ap-southeast-4" => Ok(AwsRegion::ApSoutheast4),
            "ap-southeast-5" => Ok(AwsRegion::ApSoutheast5),
            "ap-southeast-6" => Ok(AwsRegion::ApSoutheast6),
            "ap-southeast-7" => Ok(AwsRegion::ApSoutheast7),
            "ca-central-1" => Ok(AwsRegion::CaCentral1),
            "ca-west-1" => Ok(AwsRegion::CaWest1),
            "eu-central-1" => Ok(AwsRegion::EuCentral1),
            "eu-central-2" => Ok(AwsRegion::EuCentral2),
            "eu-west-1" => Ok(AwsRegion::EuWest1),
            "eu-west-2" => Ok(AwsRegion::EuWest2),
            "eu-west-3" => Ok(AwsRegion::EuWest3),
            "eu-north-1" => Ok(AwsRegion::EuNorth1),
            "eu-south-1" => Ok(AwsRegion::EuSouth1),
            "eu-south-2" => Ok(AwsRegion::EuSouth2),
            "il-central-1" => Ok(AwsRegion::IlCentral1),
            "mx-central-1" => Ok(AwsRegion::MxCentral1),
            "me-south-1" => Ok(AwsRegion::MeSouth1),
            "me-central-1" => Ok(AwsRegion::MeCentral1),
            "sa-east-1" => Ok(AwsRegion::SaEast1),
            "us-gov-east-1" => Ok(AwsRegion::UsGovEast1),
            "us-gov-west-1" => Ok(AwsRegion::UsGovWest1),
            _ => Err(format!(
                "Invalid AWS region: '{}'. See https://docs.aws.amazon.com/general/latest/gr/rande.html for valid regions",
                s
            )),
        }
    }
}
