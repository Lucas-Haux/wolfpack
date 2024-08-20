use clap:: {
    Args,
    Parser
    Subcommand
};

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct wolfpackArgs{
    /// The first argument!
    pub first_arg: String,
    /// the 2nd argu
    pub second_arg: String,
    /// 3rd arg
    pub third_arg: String,

}
