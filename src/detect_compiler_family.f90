program detect_compiler
    implicit none

#if defined(__FLANG__) || defined(__FLANG_COMPILER)
    print *, "Flang detected"
#elif defined(__LFORTRAN__)
    print *, "LFortran detected"
#elifdef __INTEL_COMPILER
    #ifdef __INTEL_COMPILER_CLASSIC
        print *, "Intel Fortran Compiler Classic (ifx) detected"
    #else
        print *, "Intel Fortran Compiler (ifort) detected"
    #endif
#elif defined(__GFORTRAN__)
    print *, "GNU Fortran Compiler detected"
#elif defined(__PGI)
    print *, "PGI Fortran Compiler detected"
#elif defined(_CRAYFTN)
    print *, "Cray Fortran Compiler detected"
#elif defined(__IBM_COMPILER)
    print *, "IBM XL Fortran Compiler detected"
#elif defined(__SUNPRO_F90)
    print *, "Oracle Solaris Studio Fortran Compiler detected"
#elif defined(__NAG__)
    print *, "NAG Fortran Compiler detected"
#else
    print *, "Unknown Fortran Compiler detected"
#endif

end program detect_compiler