import os
import click
import subprocess

"""
Given a program compiled with -ftest-coverage -fprofile-arcs or --coverage, this can be run against a corpus to generate coverage report.
Example Usage: cov.py --corpusdir ./ats_hpack_corpus --program proxy/http2/test_HPACK --title li-trafficserver (Generates the tar'd report in the corpus dir)
"""

@click.command()
@click.option("--corpusdir", required=True, help="Directory where corpus exists")
@click.option("--program",required=True, help="Program to run corpus")
@click.option("--title",default="trafficserver",help="Program to run corpus")
def run_coverage(corpusdir:str, program:str, title:str) -> bool:
    corpus_files = os.listdir(corpusdir)
    for corpus in corpus_files:
        run_program = "{} {}/{}".format(program, corpusdir, corpus)
        try:
            program_process = subprocess.run(run_program, stdout=subprocess.PIPE, shell=True, check=True)
        except:
            print("Exception caused in {}".format(run_program))
            program_process = subprocess.run("rm {}/{}.info || true".format(corpusdir, corpus), stdout=subprocess.PIPE, shell=True, check=True)
            pass

        lcov_command = "lcov --no-external --capture --directory . --output-file {}/{}.info".format(corpusdir, corpus)

        lcov_process = subprocess.run(lcov_command, stdout=subprocess.PIPE, shell=True, check=True)

    print("[+] Joining lcov info")

    info_files = [f for f in os.listdir(corpusdir) if f.endswith('.info')]
    lcov_info_a = list(map(lambda x: '-a {}/{}'.format(corpusdir, x), info_files))
    lcov_join_command = "lcov {} -o corpus.info".format(" ".join(lcov_info_a))
    print(lcov_join_command)

    lcov_process = subprocess.run(lcov_join_command, stdout=subprocess.PIPE, shell=True, check=True)

    print("[+] Generating HTML coverage report")
    genhtml_command = "genhtml --ignore-errors source corpus.info --legend --title {} --output-directory={}/coverage-{}".format(title, corpusdir, title)
    print(genhtml_command)
    generate_process = subprocess.run(genhtml_command, stdout=subprocess.PIPE, shell=True, check=True)


    print("[+] Generating tar for the report")
    zip_command = "cd {} && tar -czvf coverage-{}.tgz coverage-{}/".format(corpusdir, title, title)
    print(zip_command)
    zip_process = subprocess.run(zip_command, stdout=subprocess.PIPE, shell=True, check=True)

    return 0

if __name__ == '__main__':
    run_coverage()

